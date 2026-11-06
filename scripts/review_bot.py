#!/usr/bin/env python3
"""Run cubic, CodeRabbit and Macroscope on a pull request and publish their findings.

  review_bot.py run <reviewer> --base main --out result.json
  review_bot.py publish --pr 12 --sha abc --base main --results dir --run-url url
  review_bot.py set-account <reviewer>
"""

from __future__ import annotations

import argparse
import base64
import dataclasses
import hashlib
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
from collections.abc import Callable
from dataclasses import dataclass, field
from enum import StrEnum
from pathlib import Path

# Set by Actions, so the bot works in any repository that calls the workflow.
REPO = os.environ.get("GITHUB_REPOSITORY", "Nichokas/SDAT")
MARKER = "<!-- review-bot -->"
BOT_LOGIN = "github-actions[bot]"
# GitHub rejects comments over 65536 characters; the margin holds the table and footer.
MAX_FINDINGS_CHARS = 60000
SEVERITY_ORDER = ["critical", "major", "minor", "info"]


@dataclass(frozen=True)
class Finding:
    severity: str
    file: str
    line: int | None
    title: str
    body: str


class Status(StrEnum):
    COMPLETED = "completed"
    # The reviewer could not run for a reason outside this PR, so it neither passes nor blocks it.
    SKIPPED = "skipped"
    FAILED = "failed"


@dataclass(frozen=True)
class Result:
    reviewer: str
    status: Status
    findings: list[Finding] = field(default_factory=list)
    note: str = ""


@dataclass(frozen=True)
class Output:
    stdout: str
    stderr: str
    exit_code: int


def severity(raw: object) -> str:
    aliases = {
        "p0": "critical",
        "p1": "major",
        "high": "major",
        "p2": "minor",
        "medium": "minor",
        "p3": "info",
        "low": "info",
        "trivial": "info",
    }
    value = str(raw).strip().lower()
    value = aliases.get(value, value)
    return value if value in SEVERITY_ORDER else "info"


def json_object(text: str) -> dict | None:
    try:
        value = json.loads(text)
    except json.JSONDecodeError:
        return None
    return value if isinstance(value, dict) else None


def parse_cubic(reviewer: str, output: Output) -> Result:
    report = json_object(output.stdout) or {}
    issues = report.get("issues")
    if not isinstance(issues, list):
        return Result(reviewer, Status.FAILED)
    # Exit 1 means findings or an error, and a logged-out run reports `"issues": []` with `error`.
    completed = not report.get("error") and (output.exit_code == 0 or bool(issues))
    return Result(
        reviewer,
        Status.COMPLETED if completed else Status.FAILED,
        findings=[
            Finding(
                severity=severity(issue.get("priority")),
                file=issue.get("file", ""),
                line=issue.get("line"),
                title=issue.get("title", ""),
                body=issue.get("description", ""),
            )
            for issue in issues
        ],
    )


def parse_coderabbit(reviewer: str, output: Output) -> Result:
    findings = []
    completed = False
    for line in [*output.stdout.splitlines(), *output.stderr.splitlines()]:
        event = json_object(line) or {}
        if event.get("type") == "error" and event.get("errorType") == "rate_limit":
            wait = (event.get("metadata") or {}).get("waitTime")
            note = f"Out of included reviews; the next one is in {wait}." if wait else "Out of included reviews."
            return Result(reviewer, Status.SKIPPED, note=note)
        if event.get("type") == "finding":
            title = event.get("title", "")
            findings.append(
                Finding(
                    severity=severity(event.get("severity")),
                    file=event.get("fileName", ""),
                    line=event.get("startLine"),
                    title=title,
                    body=event.get("comment", "").removeprefix(f"**{title}**").strip(),
                )
            )
        elif event.get("type") == "complete":
            completed = event.get("status") in ("review_completed", "review_skipped")
    status = Status.COMPLETED if completed and output.exit_code == 0 else Status.FAILED
    return Result(reviewer, status, findings=findings)


def parse_macroscope(reviewer: str, output: Output) -> Result:
    findings = []
    status = ""
    note = ""
    for line in output.stderr.splitlines():
        key, _, value = line.partition("=")
        if key == "issue_event" and (event := json_object(value)):
            # Macroscope events have no title, so the first sentence takes that place.
            title, _, body = event.get("body", "").partition(". ")
            findings.append(
                Finding(
                    severity=severity(event.get("severity")),
                    file=event.get("path", ""),
                    line=event.get("line"),
                    title=title,
                    body=body,
                )
            )
        elif key == "issue_status":
            status = value
        elif key == "review_coverage" and (json_object(value) or {}).get("status") != "complete":
            note = "Macroscope did not check all changed code."
    # A late failure keeps the findings that already arrived, as the CLI contract says.
    return Result(
        reviewer,
        Status.COMPLETED if status == "completed" and output.exit_code == 0 else Status.FAILED,
        findings=findings,
        note=note,
    )


def login_coderabbit() -> None:
    subprocess.run(
        ["coderabbit", "auth", "login", "--api-key", os.environ["CODERABBIT_API_KEY"]],
        check=True,
    )


def restore_macroscope() -> None:
    os.environ["MACROSCOPE_CREDENTIAL_STORE"] = "file"
    target = Path.home() / ".macroscope"
    target.mkdir(mode=0o700, exist_ok=True)
    archive = base64.b64decode(os.environ["MACROSCOPE_AUTH"])
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        # The "data" filter drops directory modes, and Macroscope refuses a credentials
        # directory that is not 0700. The archive is our own secret, so "tar" is safe.
        tar.extractall(target, filter="tar")


@dataclass(frozen=True)
class Reviewer:
    label: str
    secret: str
    secret_help: str
    install: str
    command: Callable[[str], list[str]]
    parse: Callable[[str, Output], Result]
    authenticate: Callable[[], None] = lambda: None


REVIEWERS = {
    "cubic": Reviewer(
        label="cubic",
        secret="CUBIC_API_KEY",
        secret_help="https://www.cubic.dev/settings?tab=api-cli&integration=cli",
        install='curl -fsSL https://cubic.dev/install | CUBIC_INSTALL_DIR="$HOME/.local/bin" bash',
        command=lambda base: ["cubic", "review", "--base", base, "--json"],
        parse=parse_cubic,
    ),
    "coderabbit": Reviewer(
        label="CodeRabbit",
        secret="CODERABBIT_API_KEY",
        secret_help="https://app.coderabbit.ai/settings/api-keys (Agentic API key)",
        install="curl -fsSL https://cli.coderabbit.ai/install.sh | sh",
        command=lambda base: [
            "coderabbit", "review", "--agent", "--committed", "--base", base, "--deep",
        ],
        parse=parse_coderabbit,
        authenticate=login_coderabbit,
    ),
    "macroscope": Reviewer(
        label="Macroscope",
        secret="MACROSCOPE_AUTH",
        secret_help="created by `set-account macroscope`",
        install=(
            "curl -sSL https://raw.githubusercontent.com/prassoai/macroscope-local/main/install.sh"
            " | bash -s -- --yes --no-wizard"
        ),
        command=lambda base: [
            "macroscope", "codereview", "--raw", "--auto-update", "--mode", "precise",
            "--base", base,
        ],
        parse=parse_macroscope,
        authenticate=restore_macroscope,
    ),
}


def run_review(name: str, base: str) -> Result:
    reviewer = REVIEWERS[name]
    command = reviewer.command(base)
    os.environ["PATH"] += f":{Path.home() / '.local/bin'}"
    if not shutil.which(command[0]):
        (Path.home() / ".local/bin").mkdir(parents=True, exist_ok=True)
        subprocess.run(["bash", "-c", reviewer.install], check=True)
    # Without a secret the local CLI session is used, which lets this run on a laptop.
    if os.environ.get(reviewer.secret):
        reviewer.authenticate()
    process = subprocess.run(
        command, capture_output=True, text=True, stdin=subprocess.DEVNULL, check=False
    )
    # Errors stay in the job log, where GitHub masks secrets; the public comment links to it.
    print(process.stdout, process.stderr, sep="\n", file=sys.stderr)
    return reviewer.parse(name, Output(process.stdout, process.stderr, process.returncode))


def load_results(directory: Path) -> list[Result]:
    results = []
    for name in REVIEWERS:
        path = directory / f"{name}.json"
        if not path.exists():
            results.append(Result(name, Status.FAILED))
            continue
        data = json.loads(path.read_text())
        data["status"] = Status(data["status"])
        data["findings"] = [Finding(**finding) for finding in data["findings"]]
        results.append(Result(**data))
    return results


def render_finding(finding: Finding) -> str:
    location = f"{finding.file}:{finding.line}" if finding.line else finding.file
    text = f"**{finding.severity}** · `{location}` · {finding.title}\n\n{finding.body}\n"
    # A finding can quote `@scope/pkg`; the zero-width space keeps GitHub from pinging that user.
    return text.replace("@", "@\u200b")


def render(results: list[Result], sha: str, base: str, run_url: str) -> str:
    rows = []
    sections = []
    budget = MAX_FINDINGS_CHARS
    for result in results:
        label = REVIEWERS[result.reviewer].label
        count = len(result.findings)
        outcome = f"{count} finding{'' if count == 1 else 's'}" if count else "clean"
        if result.status is Status.FAILED:
            outcome = f"**failed** after {outcome}" if count else "**failed**, no review"
        elif result.status is Status.SKIPPED:
            outcome = "skipped"

        # Dropping whole findings keeps the HTML valid, which a cut at a character count would not.
        kept = []
        ordered = sorted(result.findings, key=lambda f: SEVERITY_ORDER.index(f.severity))
        for text in map(render_finding, ordered):
            if len(text) <= budget:
                budget -= len(text)
                kept.append(text)
        omitted = count - len(kept)
        note = " ".join(
            filter(None, [result.note, f"{omitted} omitted, see the result artifact." if omitted else ""])
        )

        rows.append(f"| {label} | {outcome} | {note} |")
        if kept:
            findings = "\n".join(kept)
            sections.append(f"<details><summary>{label} ({count})</summary>\n\n{findings}\n</details>")

    return "\n\n".join(
        [
            MARKER,
            f"### Review bot: `{sha[:7]}` against `{base}`",
            "\n".join(["| Reviewer | Result | Note |", "| --- | --- | --- |", *rows]),
            *sections,
            f"Comment `/review` to run again. [Logs]({run_url})",
        ]
    )


def gh_api(*args: str, body: dict | None = None) -> str:
    process = subprocess.run(
        ["gh", "api", *args],
        input=json.dumps(body) if body else None,
        capture_output=True,
        text=True,
        check=False,
    )
    if process.returncode:
        # CalledProcessError leaves stderr out of its message, which hides GitHub's error text.
        print(process.stderr, file=sys.stderr)
        process.check_returncode()
    return process.stdout


def upsert_comment(pr: int, body: str) -> None:
    pages = json.loads(
        gh_api("--paginate", "--slurp", f"repos/{REPO}/issues/{pr}/comments?per_page=100")
    )
    for comment in (comment for page in pages for comment in page):
        if comment["user"]["login"] == BOT_LOGIN and MARKER in comment["body"]:
            gh_api(
                "-X", "PATCH", f"repos/{REPO}/issues/comments/{comment['id']}",
                "--input", "-", body={"body": body},
            )
            return
    gh_api("-X", "POST", f"repos/{REPO}/issues/{pr}/comments", "--input", "-", body={"body": body})


def commentable_lines(patch: str) -> set[int]:
    """New-side lines GitHub accepts review comments on: added and context lines of the hunks."""
    lines = set()
    line = 0
    for row in patch.splitlines():
        if hunk := re.match(r"@@ -\d+(?:,\d+)? \+(\d+)", row):
            line = int(hunk.group(1))
        elif not row.startswith(("-", "\\")):
            lines.add(line)
            line += 1
    return lines


def inline_key(reviewer: str, finding: Finding) -> str:
    key = f"{reviewer}\0{finding.file}\0{finding.line}\0{finding.title}"
    return hashlib.sha256(key.encode()).hexdigest()[:16]


def inline_comments(
    results: list[Result], diff: dict[str, set[int]], posted: set[str]
) -> list[dict]:
    comments = []
    for result in results:
        label = REVIEWERS[result.reviewer].label
        for finding in result.findings:
            key = inline_key(result.reviewer, finding)
            if finding.line not in diff.get(finding.file, set()) or key in posted:
                continue
            posted.add(key)
            text = f"**{finding.severity}** · {label} · {finding.title}\n\n{finding.body}\n"
            comments.append(
                {
                    "path": finding.file,
                    "line": finding.line,
                    "side": "RIGHT",
                    "body": text.replace("@", "@\u200b") + f"\n<!-- review-bot:{key} -->",
                }
            )
    return comments


def post_inline(pr: int, sha: str, results: list[Result]) -> None:
    pages = json.loads(gh_api("--paginate", "--slurp", f"repos/{REPO}/pulls/{pr}/files?per_page=100"))
    diff = {f["filename"]: commentable_lines(f.get("patch", "")) for page in pages for f in page}
    pages = json.loads(
        gh_api("--paginate", "--slurp", f"repos/{REPO}/pulls/{pr}/comments?per_page=100")
    )
    posted = {
        match.group(1)
        for page in pages
        for comment in page
        if comment["user"]["login"] == BOT_LOGIN
        and (match := re.search(r"<!-- review-bot:(\w+) -->", comment["body"]))
    }
    comments = inline_comments(results, diff, posted)
    if comments:
        gh_api(
            "-X", "POST", f"repos/{REPO}/pulls/{pr}/reviews", "--input", "-",
            body={"commit_id": sha, "event": "COMMENT", "comments": comments},
        )


def is_clean(results: list[Result]) -> bool:
    ran = [result for result in results if result.status is not Status.SKIPPED]
    return bool(ran) and all(r.status is Status.COMPLETED and not r.findings for r in ran)


def update_approval(pr: int, sha: str, clean: bool) -> None:
    pages = json.loads(gh_api("--paginate", "--slurp", f"repos/{REPO}/pulls/{pr}/reviews?per_page=100"))
    approvals = [
        review
        for page in pages
        for review in page
        if review["user"]["login"] == BOT_LOGIN and review["state"] == "APPROVED"
    ]
    if clean:
        if not any(review["commit_id"] == sha for review in approvals):
            gh_api(
                "-X", "POST", f"repos/{REPO}/pulls/{pr}/reviews", "--input", "-",
                body={"commit_id": sha, "event": "APPROVE", "body": "Every reviewer that ran is clean."},
            )
        return
    # Without branch protection GitHub keeps an approval across pushes, so a new finding has to retract it.
    for review in approvals:
        gh_api(
            "-X", "PUT", f"repos/{REPO}/pulls/{pr}/reviews/{review['id']}/dismissals", "--input", "-",
            body={"message": f"`{sha[:7]}` is not clean."},
        )


def publish(pr: int, sha: str, base: str, results: list[Result], run_url: str) -> None:
    # Runs for older commits can still be publishing; they must not replace a newer review.
    if json.loads(gh_api(f"repos/{REPO}/pulls/{pr}"))["head"]["sha"] != sha:
        print(f"{sha[:7]} is no longer the head of #{pr}; skipping.")
        return
    upsert_comment(pr, render(results, sha, base, run_url))
    post_inline(pr, sha, results)
    update_approval(pr, sha, is_clean(results))


def set_account(name: str) -> None:
    reviewer = REVIEWERS[name]
    if name == "macroscope":
        value = macroscope_login()
        subprocess.run(
            ["gh", "secret", "set", reviewer.secret, "-R", REPO], input=value, text=True, check=True
        )
        return
    print(f"Create the key at {reviewer.secret_help}, then paste it here.")
    subprocess.run(["gh", "secret", "set", reviewer.secret, "-R", REPO], check=True)


def macroscope_login() -> str:
    # A throwaway HOME keeps the bot's account apart from the session on this machine.
    with tempfile.TemporaryDirectory() as home:
        env = {**os.environ, "HOME": home, "MACROSCOPE_CREDENTIAL_STORE": "file"}
        subprocess.run(["macroscope", "setup"], env=env, check=True)
        state = Path(home) / ".macroscope"
        archive = io.BytesIO()
        with tarfile.open(fileobj=archive, mode="w:gz") as tar:
            for file in ("credentials", "config.yaml"):
                tar.add(state / file, arcname=file)
    return base64.b64encode(archive.getvalue()).decode()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)

    run_cmd = commands.add_parser("run")
    run_cmd.add_argument("reviewer", choices=REVIEWERS)
    run_cmd.add_argument("--base", required=True)
    run_cmd.add_argument("--out", type=Path, required=True)

    publish_cmd = commands.add_parser("publish")
    publish_cmd.add_argument("--pr", type=int, required=True)
    publish_cmd.add_argument("--sha", required=True)
    publish_cmd.add_argument("--base", required=True)
    publish_cmd.add_argument("--results", type=Path, required=True)
    publish_cmd.add_argument("--run-url", required=True)

    account_cmd = commands.add_parser("set-account")
    account_cmd.add_argument("reviewer", choices=REVIEWERS)

    args = parser.parse_args()
    if args.command == "run":
        result = run_review(args.reviewer, args.base)
        args.out.write_text(json.dumps(dataclasses.asdict(result)))
    elif args.command == "publish":
        publish(args.pr, args.sha, args.base, load_results(args.results), args.run_url)
    else:
        set_account(args.reviewer)


if __name__ == "__main__":
    main()
