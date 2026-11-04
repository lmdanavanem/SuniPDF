#!/usr/bin/env python3
"""Synchronize the public SuniPDF project board with the repository.

* Sets the project Status of every AEC-xxx issue from docs/AEC_BACKLOG.csv
  (Planned -> Todo, In progress -> In Progress, Done -> Done).
* Optionally posts a project status update for a published release.

Requires PROJECT_TOKEN: a token allowed to write lmdanavanem's projects
(classic PAT with `project` scope, or fine-grained PAT with Projects read/write).

Usage:
  python scripts/sync-project.py [--dry-run] [--release-tag v0.92.1]
"""
from __future__ import annotations

import argparse
import csv
import json
import os
import re
import sys
import urllib.request
from pathlib import Path

OWNER = "lmdanavanem"
PROJECT_NUMBER = 1
REPO = "SuniPDF"
CSV_PATH = Path(__file__).resolve().parent.parent / "docs" / "AEC_BACKLOG.csv"
STATUS_MAP = {"planned": "Todo", "todo": "Todo", "in progress": "In Progress", "done": "Done"}
AEC_RE = re.compile(r"\bAEC-\d{3}\b")


def graphql(token: str, query: str, variables: dict) -> dict:
    request = urllib.request.Request(
        "https://api.github.com/graphql",
        data=json.dumps({"query": query, "variables": variables}).encode(),
        headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json",
                 "User-Agent": "sunipdf-project-sync"},
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        payload = json.load(response)
    if payload.get("errors"):
        raise RuntimeError(json.dumps(payload["errors"], indent=2))
    return payload["data"]


PROJECT_QUERY = """
query($owner: String!, $number: Int!, $after: String) {
  user(login: $owner) {
    projectV2(number: $number) {
      id
      field(name: "Status") {
        ... on ProjectV2SingleSelectField { id options { id name } }
      }
      items(first: 100, after: $after) {
        pageInfo { hasNextPage endCursor }
        nodes {
          id
          content { ... on Issue { number title repository { name } } }
          fieldValueByName(name: "Status") {
            ... on ProjectV2ItemFieldSingleSelectValue { name }
          }
        }
      }
    }
  }
}
"""

UPDATE_STATUS = """
mutation($project: ID!, $item: ID!, $field: ID!, $option: String!) {
  updateProjectV2ItemFieldValue(input: {projectId: $project, itemId: $item, fieldId: $field,
                                        value: {singleSelectOptionId: $option}}) {
    projectV2Item { id }
  }
}
"""

STATUS_UPDATE = """
mutation($project: ID!, $body: String!) {
  createProjectV2StatusUpdate(input: {projectId: $project, body: $body, status: ON_TRACK}) {
    statusUpdate { id }
  }
}
"""


def load_backlog() -> dict[str, str]:
    with CSV_PATH.open(encoding="utf-8", newline="") as handle:
        rows = csv.DictReader(handle)
        result = {}
        for row in rows:
            status = STATUS_MAP.get(row["status"].strip().lower())
            if status is None:
                raise SystemExit(f"{row['id']}: unknown status {row['status']!r} in {CSV_PATH.name}")
            result[row["id"].strip()] = status
        return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--release-tag", help="post a project status update for this release")
    args = parser.parse_args()

    token = os.environ.get("PROJECT_TOKEN", "")
    if not token:
        print("::notice::PROJECT_TOKEN is not configured; project sync skipped.")
        return 0

    backlog = load_backlog()
    project = None
    items = []
    after = None
    while True:
        data = graphql(token, PROJECT_QUERY, {"owner": OWNER, "number": PROJECT_NUMBER, "after": after})
        project = data["user"]["projectV2"]
        items.extend(project["items"]["nodes"])
        if not project["items"]["pageInfo"]["hasNextPage"]:
            break
        after = project["items"]["pageInfo"]["endCursor"]

    field = project["field"]
    options = {option["name"]: option["id"] for option in field["options"]}
    seen = set()
    changes = 0
    for item in items:
        content = item.get("content") or {}
        if content.get("repository", {}).get("name") != REPO:
            continue
        match = AEC_RE.search(content.get("title", ""))
        if not match or match.group(0) not in backlog:
            continue
        aec_id = match.group(0)
        seen.add(aec_id)
        wanted = backlog[aec_id]
        current = (item.get("fieldValueByName") or {}).get("name")
        if current == wanted:
            continue
        print(f"#{content['number']} {aec_id}: {current} -> {wanted}")
        changes += 1
        if not args.dry_run:
            graphql(token, UPDATE_STATUS, {"project": project["id"], "item": item["id"],
                                           "field": field["id"], "option": options[wanted]})

    missing = sorted(set(backlog) - seen)
    if missing:
        print(f"::warning::AEC cards without a project issue: {', '.join(missing)}")
    print(f"{changes} status change(s){' (dry run)' if args.dry_run else ''}.")

    if args.release_tag and not args.dry_run:
        version = args.release_tag.removeprefix("v")
        body = (f"**SuniPDF {version} published.**\n\n"
                f"Release: https://github.com/{OWNER}/{REPO}/releases/tag/{args.release_tag}\n"
                f"Notes: https://github.com/{OWNER}/{REPO}/blob/main/docs/RELEASE_{version}.md\n\n"
                f"Board synchronized with docs/AEC_BACKLOG.csv ({changes} status change(s)).")
        graphql(token, STATUS_UPDATE, {"project": project["id"], "body": body})
        print("Project status update posted.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
