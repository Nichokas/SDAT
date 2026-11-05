import base64
import dataclasses
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
    Status,
    commentable_lines,
    inline_comments,
    inline_key,
    is_clean,
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
        self.assertIs(result.status, Status.COMPLETED)
        self.assertEqual(result.findings, [Finding("critical", "a.rs", 3, "t", "d")])

    def test_cubic_error_is_not_clean(self):
        self.assertIs(parse_cubic("cubic", Output("Unauthorized", "", 1)).status, Status.FAILED)

    def test_cubic_logged_out_is_not_clean(self):
        stdout = json.dumps({"issues": [], "error": "No AI provider is configured."})
        self.assertIs(parse_cubic("cubic", Output(stdout, "", 1)).status, Status.FAILED)

    def test_cubic_empty_issues_with_exit_one_is_not_clean(self):
        self.assertIs(parse_cubic("cubic", Output('{"issues": []}', "", 1)).status, Status.FAILED)

    def test_cubic_clean_review_is_clean(self):
        self.assertIs(parse_cubic("cubic", Output('{"issues": []}', "", 0)).status, Status.COMPLETED)

    def test_coderabbit_needs_the_completion_event(self):
        event = {"type": "finding", "severity": "major", "fileName": "a.rs", "startLine": 3,
                 "title": "t", "comment": "**t**\n\nd"}
        complete = {"type": "complete", "status": "review_completed"}
        lines = [json.dumps(event), '{"type":"heartbeat"}', "not json"]

        cut_short = parse_coderabbit("coderabbit", Output("\n".join(lines), "", 0))
        self.assertIs(cut_short.status, Status.FAILED)

        done = parse_coderabbit("coderabbit", Output("\n".join([*lines, json.dumps(complete)]), "", 0))
        self.assertIs(done.status, Status.COMPLETED)
        self.assertEqual(done.findings, [Finding("major", "a.rs", 3, "t", "d")])

    def test_coderabbit_no_changes_is_a_finished_review(self):
        stdout = json.dumps({"type": "complete", "status": "review_skipped", "findings": 0})
        self.assertIs(parse_coderabbit("coderabbit", Output(stdout, "", 0)).status, Status.COMPLETED)

    def test_coderabbit_rate_limit_is_skipped(self):
        event = {"type": "error", "errorType": "rate_limit", "metadata": {"waitTime": "24 minutes"}}
        result = parse_coderabbit("coderabbit", Output(json.dumps(event), "", 1))
        self.assertIs(result.status, Status.SKIPPED)
        self.assertIn("24 minutes", result.note)

    def test_coderabbit_other_error_still_fails(self):
        event = {"type": "error", "errorType": "auth", "message": "bad key"}
        self.assertIs(parse_coderabbit("coderabbit", Output(json.dumps(event), "", 1)).status, Status.FAILED)

    def test_macroscope_failure_keeps_findings(self):
        event = {"path": "a.rs", "line": 3, "severity": "medium", "body": "Bad thing. More detail."}
        stderr = "\n".join(
            ["review_session_id=x", "issue_event={broken", f"issue_event={json.dumps(event)}",
             "issue_status=failed"]
        )
        result = parse_macroscope("macroscope", Output("", stderr, 1))
        self.assertIs(result.status, Status.FAILED)
        self.assertEqual(result.findings, [Finding("minor", "a.rs", 3, "Bad thing", "More detail.")])

    def test_macroscope_nonzero_exit_overrides_completed_status(self):
        self.assertIs(parse_macroscope("macroscope", Output("", "issue_status=completed", 1)).status, Status.FAILED)

    def test_macroscope_flags_partial_coverage(self):
        stderr = 'review_coverage={"status":"partial"}\nissue_status=completed'
        result = parse_macroscope("macroscope", Output("", stderr, 0))
        self.assertIs(result.status, Status.COMPLETED)
        self.assertIn("did not check", result.note)


class RenderTests(unittest.TestCase):
    def render(self, *results):
        return render(list(results), "abcdef123", "main", "https://run")

    def test_failed_reviewer_is_never_clean(self):
        comment = self.render(Result("cubic", Status.FAILED), Result("coderabbit", Status.COMPLETED))
        self.assertIn("| cubic | **failed**, no review |", comment)
        self.assertIn("| CodeRabbit | clean |", comment)

    def test_skipped_reviewer_says_so(self):
        comment = self.render(Result("coderabbit", Status.SKIPPED, note="Out of included reviews."))
        self.assertIn("| CodeRabbit | skipped | Out of included reviews. |", comment)

    def test_mentions_do_not_ping(self):
        comment = self.render(Result("cubic", Status.COMPLETED, [finding("uses @types/node")]))
        self.assertNotIn("@types", comment)

    def test_oversized_review_stays_within_github_limit_and_balanced(self):
        huge = [finding("x" * 30000) for _ in range(5)]
        comment = self.render(Result("cubic", Status.COMPLETED, huge))
        self.assertLess(len(comment), 65536)
        self.assertEqual(comment.count("<details>"), comment.count("</details>"))
        self.assertIn("4 omitted", comment)
        self.assertIn("/review", comment)

    def test_most_severe_finding_comes_first(self):
        comment = self.render(
            Result("cubic", Status.COMPLETED, [finding("low one", "info"), finding("high one", "critical")])
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


class InlineTests(unittest.TestCase):
    def test_only_new_side_lines_of_hunks_are_commentable(self):
        patch = "@@ -1,3 +1,4 @@\n keep\n-old\n+new\n+added\n keep\n@@ -20 +21,2 @@ fn x\n ctx\n+tail\n\\ No newline at end of file"
        self.assertEqual(commentable_lines(patch), {1, 2, 3, 4, 21, 22})

    def test_findings_outside_the_diff_or_already_posted_are_skipped(self):
        on_diff = Finding("major", "a.rs", 3, "on diff", "ping @someone")
        off_diff = Finding("major", "a.rs", 9, "off diff", "b")
        other_file = Finding("major", "b.rs", 3, "other file", "b")
        no_line = Finding("major", "a.rs", None, "no line", "b")
        old = Finding("major", "a.rs", 3, "already posted", "b")
        results = [Result("cubic", Status.COMPLETED, [on_diff, off_diff, other_file, no_line, old, on_diff])]

        comments = inline_comments(results, {"a.rs": {3}}, {inline_key("cubic", old)})

        self.assertEqual(len(comments), 1)
        self.assertEqual((comments[0]["path"], comments[0]["line"]), ("a.rs", 3))
        self.assertIn(f"<!-- review-bot:{inline_key('cubic', on_diff)} -->", comments[0]["body"])
        self.assertNotIn("@someone", comments[0]["body"])


    def test_same_title_on_two_lines_gets_two_comments(self):
        findings = [Finding("major", "a.rs", 3, "same", "b"), Finding("major", "a.rs", 4, "same", "b")]
        comments = inline_comments([Result("cubic", Status.COMPLETED, findings)], {"a.rs": {3, 4}}, set())
        self.assertEqual([c["line"] for c in comments], [3, 4])


class PublishTests(unittest.TestCase):
    def test_stale_run_does_not_publish(self):
        calls = []

        def fake(*args, body=None):
            calls.append(args)
            return json.dumps({"head": {"sha": "newer"}})

        with mock.patch.object(review_bot, "gh_api", fake):
            review_bot.publish(7, "older", "main", [Result("cubic", Status.COMPLETED)], "url")
        self.assertEqual(calls, [(f"repos/{review_bot.REPO}/pulls/7",)])

    def test_publish_command_reaches_publish(self):
        with tempfile.TemporaryDirectory() as results, mock.patch.object(
            review_bot, "publish"
        ) as publish, mock.patch(
            "sys.argv",
            ["review_bot.py", "publish", "--pr", "7", "--sha", "abc", "--base", "main",
             "--results", results, "--run-url", "url"],
        ):
            review_bot.main()
        self.assertEqual(publish.call_args.args[:3], (7, "abc", "main"))


class ApprovalTests(unittest.TestCase):
    clean = Result("cubic", Status.COMPLETED)
    skipped = Result("coderabbit", Status.SKIPPED)

    def test_skipped_reviewer_does_not_block_approval(self):
        self.assertTrue(is_clean([self.clean, self.skipped]))

    def test_findings_failures_or_no_review_at_all_block_approval(self):
        found = Result("macroscope", Status.COMPLETED, [finding()])
        failed = Result("macroscope", Status.FAILED)
        self.assertFalse(is_clean([self.clean, self.skipped, found]))
        self.assertFalse(is_clean([self.clean, self.skipped, failed]))
        self.assertFalse(is_clean([self.skipped]))

    def update(self, clean, reviews):
        calls = []

        def fake(*args, body=None):
            calls.append((args, body))
            return json.dumps([reviews]) if "--paginate" in args else ""

        with mock.patch.object(review_bot, "gh_api", fake):
            review_bot.update_approval(7, "abc", clean)
        return calls[1:]

    def approval(self, sha, login=review_bot.BOT_LOGIN):
        return {"id": 5, "user": {"login": login}, "state": "APPROVED", "commit_id": sha}

    def test_clean_run_approves_the_head(self):
        [(args, body)] = self.update(True, [self.approval("older")])
        self.assertIn(f"repos/{review_bot.REPO}/pulls/7/reviews", args)
        self.assertEqual((body["event"], body["commit_id"]), ("APPROVE", "abc"))

    def test_rerun_on_an_approved_head_does_not_approve_twice(self):
        self.assertEqual(self.update(True, [self.approval("abc")]), [])

    def test_unclean_run_dismisses_only_the_bot_approvals(self):
        [(args, body)] = self.update(False, [self.approval("older"), self.approval("x", "someone")])
        self.assertIn(f"repos/{review_bot.REPO}/pulls/7/reviews/5/dismissals", args)
        self.assertIn("abc", body["message"])

    def test_status_survives_the_result_artifact(self):
        with tempfile.TemporaryDirectory() as directory:
            for result in (self.clean, self.skipped):
                path = Path(directory) / f"{result.reviewer}.json"
                path.write_text(json.dumps(dataclasses.asdict(result)))
            results = review_bot.load_results(Path(directory))
        self.assertEqual([r.status for r in results], [Status.COMPLETED, Status.SKIPPED, Status.FAILED])
        self.assertTrue(is_clean(results[:2]))


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
