import base64
import io
import json
import os
import tarfile
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import review_bot
from review_bot import (
    Finding,
    Output,
    Result,
    parse_coderabbit,
    parse_cubic,
    parse_macroscope,
    render,
)


def finding(body="body", severity="minor"):
    return Finding(severity, "a.rs", 3, "title", body)


class ParseTests(unittest.TestCase):
    def test_cubic_reports_findings_despite_exit_one(self):
        stdout = json.dumps(
            {"issues": [{"priority": "P0", "file": "a.rs", "line": 3, "title": "t", "description": "d"}]}
        )
        result = parse_cubic("cubic", Output(stdout, "", 1))
        self.assertTrue(result.completed)
        self.assertEqual(result.findings, [Finding("critical", "a.rs", 3, "t", "d")])

    def test_cubic_error_is_not_clean(self):
        self.assertFalse(parse_cubic("cubic", Output("Unauthorized", "", 1)).completed)

    def test_cubic_logged_out_is_not_clean(self):
        stdout = json.dumps({"issues": [], "error": "No AI provider is configured."})
        self.assertFalse(parse_cubic("cubic", Output(stdout, "", 1)).completed)

    def test_cubic_empty_issues_with_exit_one_is_not_clean(self):
        self.assertFalse(parse_cubic("cubic", Output('{"issues": []}', "", 1)).completed)

    def test_cubic_clean_review_is_clean(self):
        self.assertTrue(parse_cubic("cubic", Output('{"issues": []}', "", 0)).completed)

    def test_coderabbit_needs_the_completion_event(self):
        event = {"type": "finding", "severity": "major", "fileName": "a.rs", "startLine": 3,
                 "title": "t", "comment": "**t**\n\nd"}
        complete = {"type": "complete", "status": "review_completed"}
        lines = [json.dumps(event), '{"type":"heartbeat"}', "not json"]

        cut_short = parse_coderabbit("coderabbit", Output("\n".join(lines), "", 0))
        self.assertFalse(cut_short.completed)

        done = parse_coderabbit("coderabbit", Output("\n".join([*lines, json.dumps(complete)]), "", 0))
        self.assertTrue(done.completed)
        self.assertEqual(done.findings, [Finding("major", "a.rs", 3, "t", "d")])

    def test_coderabbit_no_changes_is_a_finished_review(self):
        stdout = json.dumps({"type": "complete", "status": "review_skipped", "findings": 0})
        self.assertTrue(parse_coderabbit("coderabbit", Output(stdout, "", 0)).completed)

    def test_macroscope_failure_keeps_findings(self):
        event = {"path": "a.rs", "line": 3, "severity": "medium", "body": "Bad thing. More detail."}
        stderr = "\n".join(
            ["review_session_id=x", "issue_event={broken", f"issue_event={json.dumps(event)}",
             "issue_status=failed"]
        )
        result = parse_macroscope("macroscope", Output("", stderr, 1))
        self.assertFalse(result.completed)
        self.assertEqual(result.findings, [Finding("minor", "a.rs", 3, "Bad thing", "More detail.")])

    def test_macroscope_nonzero_exit_overrides_completed_status(self):
        self.assertFalse(parse_macroscope("macroscope", Output("", "issue_status=completed", 1)).completed)

    def test_macroscope_flags_partial_coverage(self):
        stderr = 'review_coverage={"status":"partial"}\nissue_status=completed'
        result = parse_macroscope("macroscope", Output("", stderr, 0))
        self.assertTrue(result.completed)
        self.assertIn("did not check", result.note)


class RenderTests(unittest.TestCase):
    def render(self, *results):
        return render(list(results), "abcdef123", "main", "https://run")

    def test_failed_reviewer_is_never_clean(self):
        comment = self.render(Result("cubic", completed=False), Result("coderabbit", completed=True))
        self.assertIn("| cubic | **failed**, no review |", comment)
        self.assertIn("| CodeRabbit | clean |", comment)

    def test_mentions_do_not_ping(self):
        comment = self.render(Result("cubic", True, [finding("uses @types/node")]))
        self.assertNotIn("@types", comment)

    def test_oversized_review_stays_within_github_limit_and_balanced(self):
        huge = [finding("x" * 30000) for _ in range(5)]
        comment = self.render(Result("cubic", True, huge))
        self.assertLess(len(comment), 65536)
        self.assertEqual(comment.count("<details>"), comment.count("</details>"))
        self.assertIn("4 omitted", comment)
        self.assertIn("/review", comment)

    def test_most_severe_finding_comes_first(self):
        comment = self.render(
            Result("cubic", True, [finding("low one", "info"), finding("high one", "critical")])
        )
        self.assertLess(comment.index("high one"), comment.index("low one"))


class UpsertTests(unittest.TestCase):
    def upsert(self, comments):
        calls = []

        def fake(*args, body=None):
            calls.append(args)
            return json.dumps([comments]) if "--paginate" in args else ""

        with mock.patch.object(review_bot, "gh_api", fake):
            review_bot.upsert_comment(7, f"{review_bot.MARKER} new")
        return calls[-1]

    def test_rerun_updates_the_existing_comment(self):
        call = self.upsert(
            [{"id": 42, "user": {"login": review_bot.BOT_LOGIN}, "body": f"{review_bot.MARKER} old"}]
        )
        self.assertIn("PATCH", call)
        self.assertIn(f"repos/{review_bot.REPO}/issues/comments/42", call)

    def test_quoted_marker_from_a_person_is_left_alone(self):
        call = self.upsert([{"id": 42, "user": {"login": "someone"}, "body": review_bot.MARKER}])
        self.assertIn("POST", call)


class MacroscopeRestoreTests(unittest.TestCase):
    def test_credentials_directory_keeps_owner_only_mode(self):
        with tempfile.TemporaryDirectory() as login, tempfile.TemporaryDirectory() as home:
            credentials = Path(login) / "credentials"
            credentials.mkdir(mode=0o700)
            (credentials / "session").write_text("token")
            (Path(login) / "config.yaml").write_text("workspace: x")
            archive = io.BytesIO()
            with tarfile.open(fileobj=archive, mode="w:gz") as tar:
                for name in ("credentials", "config.yaml"):
                    tar.add(Path(login) / name, arcname=name)
            auth = base64.b64encode(archive.getvalue()).decode()

            with mock.patch.dict(os.environ, {"HOME": home, "MACROSCOPE_AUTH": auth}):
                review_bot.restore_macroscope()

            restored = Path(home) / ".macroscope" / "credentials"
            self.assertEqual(restored.stat().st_mode & 0o777, 0o700)
            self.assertEqual((restored / "session").read_text(), "token")


if __name__ == "__main__":
    unittest.main()
