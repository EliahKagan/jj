// Copyright 2022 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::common::create_commit;
use crate::common::CommandOutput;
use crate::common::TestEnvironment;
use crate::common::TestWorkDir;

#[test]
fn test_rebase_invalid() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b", &["a"]);

    // Missing destination
    let output = work_dir.run_jj(["rebase"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    error: the following required arguments were not provided:
      <--destination <REVSETS>|--insert-after <REVSETS>|--insert-before <REVSETS>>

    Usage: jj rebase <--destination <REVSETS>|--insert-after <REVSETS>|--insert-before <REVSETS>>

    For more information, try '--help'.
    [EOF]
    [exit status: 2]
    ");

    // Both -r and -s
    let output = work_dir.run_jj(["rebase", "-r", "a", "-s", "a", "-d", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    error: the argument '--revisions <REVSETS>' cannot be used with '--source <REVSETS>'

    Usage: jj rebase --revisions <REVSETS> <--destination <REVSETS>|--insert-after <REVSETS>|--insert-before <REVSETS>>

    For more information, try '--help'.
    [EOF]
    [exit status: 2]
    ");

    // Both -b and -s
    let output = work_dir.run_jj(["rebase", "-b", "a", "-s", "a", "-d", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    error: the argument '--branch <REVSETS>' cannot be used with '--source <REVSETS>'

    Usage: jj rebase --branch <REVSETS> <--destination <REVSETS>|--insert-after <REVSETS>|--insert-before <REVSETS>>

    For more information, try '--help'.
    [EOF]
    [exit status: 2]
    ");

    // Both -d and --after
    let output = work_dir.run_jj(["rebase", "-r", "a", "-d", "b", "--after", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    error: the argument '--destination <REVSETS>' cannot be used with '--insert-after <REVSETS>'

    Usage: jj rebase --revisions <REVSETS> <--destination <REVSETS>|--insert-after <REVSETS>|--insert-before <REVSETS>>

    For more information, try '--help'.
    [EOF]
    [exit status: 2]
    ");

    // Both -d and --before
    let output = work_dir.run_jj(["rebase", "-r", "a", "-d", "b", "--before", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    error: the argument '--destination <REVSETS>' cannot be used with '--insert-before <REVSETS>'

    Usage: jj rebase --revisions <REVSETS> <--destination <REVSETS>|--insert-after <REVSETS>|--insert-before <REVSETS>>

    For more information, try '--help'.
    [EOF]
    [exit status: 2]
    ");

    // Rebase onto self with -r
    let output = work_dir.run_jj(["rebase", "-r", "a", "-d", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Cannot rebase 2443ea76b0b1 onto itself
    [EOF]
    [exit status: 1]
    ");

    // Rebase root with -r
    let output = work_dir.run_jj(["rebase", "-r", "root()", "-d", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: The root commit 000000000000 is immutable
    [EOF]
    [exit status: 1]
    ");

    // Rebase onto descendant with -s
    let output = work_dir.run_jj(["rebase", "-s", "a", "-d", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Cannot rebase 2443ea76b0b1 onto descendant 1394f625cbbd
    [EOF]
    [exit status: 1]
    ");
}

#[test]
fn test_rebase_empty_sets() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b", &["a"]);

    // TODO: Make all of these say "Nothing changed"?
    let output = work_dir.run_jj(["rebase", "-r=none()", "-d=b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Nothing changed.
    [EOF]
    ");
    let output = work_dir.run_jj(["rebase", "-s=none()", "-d=b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Revset `none()` didn't resolve to any revisions
    [EOF]
    [exit status: 1]
    ");
    let output = work_dir.run_jj(["rebase", "-b=none()", "-d=b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Revset `none()` didn't resolve to any revisions
    [EOF]
    [exit status: 1]
    ");
    // Empty because "b..a" is empty
    let output = work_dir.run_jj(["rebase", "-b=a", "-d=b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Nothing changed.
    [EOF]
    ");
}

#[test]
fn test_rebase_bookmark() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b", &["a"]);
    create_commit(&work_dir, "c", &["b"]);
    create_commit(&work_dir, "d", &["b"]);
    create_commit(&work_dir, "e", &["a"]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  e: a
    │ ○  d: b
    │ │ ○  c: b
    │ ├─╯
    │ ○  b: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");

    let output = work_dir.run_jj(["rebase", "-b", "c", "-d", "e"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○  d: b
    │ ○  c: b
    ├─╯
    ○  b: e
    @  e: a
    ○  a
    ◆
    [EOF]
    ");

    // Test rebasing multiple bookmarks at once
    work_dir.run_jj(["undo"]).success();
    let output = work_dir.run_jj(["rebase", "-b=e", "-b=d", "-d=b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 1 commits that were already in place
    Rebased 1 commits onto destination
    Working copy  (@) now at: znkkpsqq 9ca2a154 e | e
    Parent commit (@-)      : zsuskuln 1394f625 b | b
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  e: b
    │ ○  d: b
    ├─╯
    │ ○  c: b
    ├─╯
    ○  b: a
    ○  a
    ◆
    [EOF]
    ");

    // Same test but with more than one revision per argument
    work_dir.run_jj(["undo"]).success();
    let output = work_dir.run_jj(["rebase", "-b=e|d", "-d=b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Revset `e|d` resolved to more than one revision
    Hint: The revset `e|d` resolved to these revisions:
      znkkpsqq e52756c8 e | e
      vruxwmqv 514fa6b2 d | d
    Hint: Prefix the expression with `all:` to allow any number of revisions (i.e. `all:e|d`).
    [EOF]
    [exit status: 1]
    ");
    let output = work_dir.run_jj(["rebase", "-b=all:e|d", "-d=b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 1 commits that were already in place
    Rebased 1 commits onto destination
    Working copy  (@) now at: znkkpsqq 817e3fb0 e | e
    Parent commit (@-)      : zsuskuln 1394f625 b | b
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  e: b
    │ ○  d: b
    ├─╯
    │ ○  c: b
    ├─╯
    ○  b: a
    ○  a
    ◆
    [EOF]
    ");
}

#[test]
fn test_rebase_bookmark_with_merge() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b", &["a"]);
    create_commit(&work_dir, "c", &[]);
    create_commit(&work_dir, "d", &["c"]);
    create_commit(&work_dir, "e", &["a", "d"]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    e: a d
    ├─╮
    │ ○  d: c
    │ ○  c
    │ │ ○  b: a
    ├───╯
    ○ │  a
    ├─╯
    ◆
    [EOF]
    ");

    let output = work_dir.run_jj(["rebase", "-b", "d", "-d", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Working copy  (@) now at: znkkpsqq 5f8a3db2 e | e
    Parent commit (@-)      : rlvkpnrz 2443ea76 a | a
    Parent commit (@-)      : vruxwmqv 1677f795 d | d
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    e: a d
    ├─╮
    │ ○  d: c
    │ ○  c: b
    │ ○  b: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");

    work_dir.run_jj(["undo"]).success();
    let output = work_dir.run_jj(["rebase", "-d", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Working copy  (@) now at: znkkpsqq a331ac11 e | e
    Parent commit (@-)      : rlvkpnrz 2443ea76 a | a
    Parent commit (@-)      : vruxwmqv 3d0f3644 d | d
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    e: a d
    ├─╮
    │ ○  d: c
    │ ○  c: b
    │ ○  b: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
}

#[test]
fn test_rebase_single_revision() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b", &["a"]);
    create_commit(&work_dir, "c", &["a"]);
    create_commit(&work_dir, "d", &["b", "c"]);
    create_commit(&work_dir, "e", &["d"]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  e: d
    ○    d: b c
    ├─╮
    │ ○  c: a
    ○ │  b: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");

    // Descendants of the rebased commit "c" should be rebased onto parents. First
    // we test with a non-merge commit.
    let output = work_dir.run_jj(["rebase", "-r", "c", "-d", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 2 descendant commits
    Working copy  (@) now at: znkkpsqq 2668ffbe e | e
    Parent commit (@-)      : vruxwmqv 7b370c85 d | d
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  e: d
    ○    d: b a
    ├─╮
    │ │ ○  c: b
    ├───╯
    ○ │  b: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["undo"]).success();

    // Now, let's try moving the merge commit. After, both parents of "d" ("b" and
    // "c") should become parents of "e".
    let output = work_dir.run_jj(["rebase", "-r", "d", "-d", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 1 descendant commits
    Working copy  (@) now at: znkkpsqq ed210c15 e | e
    Parent commit (@-)      : zsuskuln 1394f625 b | b
    Parent commit (@-)      : royxmykx c0cb3a0b c | c
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    e: b c
    ├─╮
    │ ○  c: a
    ○ │  b: a
    ├─╯
    │ ○  d: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
}

#[test]
fn test_rebase_single_revision_merge_parent() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b", &[]);
    create_commit(&work_dir, "c", &["b"]);
    create_commit(&work_dir, "d", &["a", "c"]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    d: a c
    ├─╮
    │ ○  c: b
    │ ○  b
    ○ │  a
    ├─╯
    ◆
    [EOF]
    ");

    // Descendants of the rebased commit should be rebased onto parents, and if
    // the descendant is a merge commit, it shouldn't forget its other parents.
    let output = work_dir.run_jj(["rebase", "-r", "c", "-d", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 1 descendant commits
    Working copy  (@) now at: vruxwmqv a37531e8 d | d
    Parent commit (@-)      : rlvkpnrz 2443ea76 a | a
    Parent commit (@-)      : zsuskuln d370aee1 b | b
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    d: a b
    ├─╮
    │ ○  b
    │ │ ○  c: a
    ├───╯
    ○ │  a
    ├─╯
    ◆
    [EOF]
    ");
}

#[test]
fn test_rebase_multiple_revisions() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b", &["a"]);
    create_commit(&work_dir, "c", &["b"]);
    create_commit(&work_dir, "d", &["a"]);
    create_commit(&work_dir, "e", &["d"]);
    create_commit(&work_dir, "f", &["c", "e"]);
    create_commit(&work_dir, "g", &["f"]);
    create_commit(&work_dir, "h", &["g"]);
    create_commit(&work_dir, "i", &["f"]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  i: f
    │ ○  h: g
    │ ○  g: f
    ├─╯
    ○    f: c e
    ├─╮
    │ ○  e: d
    │ ○  d: a
    ○ │  c: b
    ○ │  b: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");

    // Test with two non-related non-merge commits.
    let output = work_dir.run_jj(["rebase", "-r", "c", "-r", "e", "-d", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 commits onto destination
    Rebased 4 descendant commits
    Working copy  (@) now at: xznxytkn 016685dc i | i
    Parent commit (@-)      : kmkuslsw e04d3932 f | f
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  i: f
    │ ○  h: g
    │ ○  g: f
    ├─╯
    ○    f: b d
    ├─╮
    │ ○  d: a
    ○ │  b: a
    ├─╯
    │ ○  e: a
    ├─╯
    │ ○  c: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["undo"]).success();

    // Test with two related non-merge commits. Since "b" is a parent of "c", when
    // rebasing commits "b" and "c", their ancestry relationship should be
    // preserved.
    let output = work_dir.run_jj(["rebase", "-r", "b", "-r", "c", "-d", "e"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 commits onto destination
    Rebased 4 descendant commits
    Working copy  (@) now at: xznxytkn 94538385 i | i
    Parent commit (@-)      : kmkuslsw dae8d293 f | f
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  i: f
    │ ○  h: g
    │ ○  g: f
    ├─╯
    ○    f: a e
    ├─╮
    │ │ ○  c: b
    │ │ ○  b: e
    │ ├─╯
    │ ○  e: d
    │ ○  d: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["undo"]).success();

    // Test with a subgraph containing a merge commit. Since the merge commit "f"
    // was extracted, its descendants which are not part of the subgraph will
    // inherit its descendants which are not in the subtree ("c" and "d").
    // "f" will retain its parent "c" since "c" is outside the target set, and not
    // a descendant of any new children.
    let output = work_dir.run_jj(["rebase", "-r", "e::g", "-d", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Rebased 2 descendant commits
    Working copy  (@) now at: xznxytkn 1868ded4 i | i
    Parent commit (@-)      : royxmykx 7e4fbf4f c | c
    Parent commit (@-)      : vruxwmqv 4cc44fbf d | d
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    i: c d
    ├─╮
    │ │ ○  h: c d
    ╭─┬─╯
    │ ○  d: a
    │ │ ○  g: f
    │ │ ○  f: c e
    ╭───┤
    │ │ ○  e: a
    │ ├─╯
    ○ │  c: b
    ○ │  b: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["undo"]).success();

    // Test with commits in a disconnected subgraph. The subgraph has the
    // relationship d->e->f->g->h, but only "d", "f" and "h" are in the set of
    // rebased commits. "d" should be a new parent of "f", and "f" should be a
    // new parent of "h". "f" will retain its parent "c" since "c" is outside the
    // target set, and not a descendant of any new children.
    let output = work_dir.run_jj(["rebase", "-r", "d", "-r", "f", "-r", "h", "-d", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Rebased 3 descendant commits
    Working copy  (@) now at: xznxytkn 9cfd1635 i | i
    Parent commit (@-)      : royxmykx 7e4fbf4f c | c
    Parent commit (@-)      : znkkpsqq ecf9a1d5 e | e
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    i: c e
    ├─╮
    │ │ ○  g: c e
    ╭─┬─╯
    │ ○  e: a
    │ │ ○  h: f
    │ │ ○  f: c d
    ╭───┤
    │ │ ○  d: b
    ○ │ │  c: b
    ├───╯
    ○ │  b: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["undo"]).success();

    // Test rebasing a subgraph onto its descendants.
    let output = work_dir.run_jj(["rebase", "-r", "d::e", "-d", "i"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 commits onto destination
    Rebased 4 descendant commits
    Working copy  (@) now at: xznxytkn 5d911e5c i | i
    Parent commit (@-)      : kmkuslsw d1bfda8c f | f
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○  e: d
    ○  d: i
    @  i: f
    │ ○  h: g
    │ ○  g: f
    ├─╯
    ○    f: c a
    ├─╮
    ○ │  c: b
    ○ │  b: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
}

#[test]
fn test_rebase_revision_onto_descendant() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "base", &[]);
    create_commit(&work_dir, "a", &["base"]);
    create_commit(&work_dir, "b", &["base"]);
    create_commit(&work_dir, "merge", &["b", "a"]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    merge: b a
    ├─╮
    │ ○  a: base
    ○ │  b: base
    ├─╯
    ○  base
    ◆
    [EOF]
    ");
    let setup_opid = work_dir.current_operation_id();

    // Simpler example
    let output = work_dir.run_jj(["rebase", "-r", "base", "-d", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 3 descendant commits
    Working copy  (@) now at: vruxwmqv bff4a4eb merge | merge
    Parent commit (@-)      : royxmykx c84e900d b | b
    Parent commit (@-)      : zsuskuln d57db87b a | a
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    merge: b a
    ├─╮
    ○ │  b
    │ │ ○  base: a
    │ ├─╯
    │ ○  a
    ├─╯
    ◆
    [EOF]
    ");

    // Now, let's rebase onto the descendant merge
    let output = work_dir.run_jj(["op", "restore", &setup_opid]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Restored to operation: 60e340690be6 (2001-02-03 08:05:15) create bookmark merge pointing to commit b05964d109522cd06e48f1a2661e1a0f58be0984
    Working copy  (@) now at: vruxwmqv b05964d1 merge | merge
    Parent commit (@-)      : royxmykx cea87a87 b | b
    Parent commit (@-)      : zsuskuln 2c5b7858 a | a
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    let output = work_dir.run_jj(["rebase", "-r", "base", "-d", "merge"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 3 descendant commits
    Working copy  (@) now at: vruxwmqv 986b7a49 merge | merge
    Parent commit (@-)      : royxmykx c07c677c b | b
    Parent commit (@-)      : zsuskuln abc90087 a | a
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○  base: merge
    @    merge: b a
    ├─╮
    │ ○  a
    ○ │  b
    ├─╯
    ◆
    [EOF]
    ");

    // TODO(ilyagr): These will be good tests for `jj rebase --insert-after` and
    // `--insert-before`, once those are implemented.
}

#[test]
fn test_rebase_multiple_destinations() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b", &[]);
    create_commit(&work_dir, "c", &[]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c
    │ ○  b
    ├─╯
    │ ○  a
    ├─╯
    ◆
    [EOF]
    ");

    let output = work_dir.run_jj(["rebase", "-r", "a", "-d", "b", "-d", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○    a: b c
    ├─╮
    │ @  c
    ○ │  b
    ├─╯
    ◆
    [EOF]
    ");

    let output = work_dir.run_jj(["rebase", "-r", "a", "-d", "b|c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Revset `b|c` resolved to more than one revision
    Hint: The revset `b|c` resolved to these revisions:
      royxmykx fe2e8e8b c | c
      zsuskuln d370aee1 b | b
    Hint: Prefix the expression with `all:` to allow any number of revisions (i.e. `all:b|c`).
    [EOF]
    [exit status: 1]
    ");

    // try with 'all:' and succeed
    let output = work_dir.run_jj(["rebase", "-r", "a", "-d", "all:b|c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○    a: c b
    ├─╮
    │ ○  b
    @ │  c
    ├─╯
    ◆
    [EOF]
    ");

    // undo and do it again, but with 'ui.always-allow-large-revsets'
    work_dir.run_jj(["undo"]).success();
    work_dir
        .run_jj([
            "rebase",
            "--config=ui.always-allow-large-revsets=true",
            "-r=a",
            "-d=b|c",
        ])
        .success();
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○    a: c b
    ├─╮
    │ ○  b
    @ │  c
    ├─╯
    ◆
    [EOF]
    ");

    let output = work_dir.run_jj(["rebase", "-r", "a", "-d", "b", "-d", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: More than one revset resolved to revision d370aee184ba
    [EOF]
    [exit status: 1]
    ");

    // Same error with 'all:' if there is overlap.
    let output = work_dir.run_jj(["rebase", "-r", "a", "-d", "all:b|c", "-d", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: More than one revset resolved to revision d370aee184ba
    [EOF]
    [exit status: 1]
    ");

    let output = work_dir.run_jj(["rebase", "-r", "a", "-d", "b", "-d", "root()"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: The Git backend does not support creating merge commits with the root commit as one of the parents.
    [EOF]
    [exit status: 1]
    ");
}

#[test]
fn test_rebase_with_descendants() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b", &[]);
    create_commit(&work_dir, "c", &["a", "b"]);
    create_commit(&work_dir, "d", &["c"]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  d: c
    ○    c: a b
    ├─╮
    │ ○  b
    ○ │  a
    ├─╯
    ◆
    [EOF]
    ");

    let output = work_dir.run_jj(["rebase", "-s", "b", "-d", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Working copy  (@) now at: vruxwmqv 705832bd d | d
    Parent commit (@-)      : royxmykx 57c7246a c | c
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  d: c
    ○    c: a b
    ├─╮
    │ ○  b: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");

    // Rebase several subtrees at once.
    work_dir.run_jj(["undo"]).success();
    let output = work_dir.run_jj(["rebase", "-s=c", "-s=d", "-d=a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 commits onto destination
    Working copy  (@) now at: vruxwmqv 92c2bc9a d | d
    Parent commit (@-)      : rlvkpnrz 2443ea76 a | a
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  d: a
    │ ○  c: a
    ├─╯
    ○  a
    │ ○  b
    ├─╯
    ◆
    [EOF]
    ");

    work_dir.run_jj(["undo"]).success();
    // Reminder of the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  d: c
    ○    c: a b
    ├─╮
    │ ○  b
    ○ │  a
    ├─╯
    ◆
    [EOF]
    ");

    // `d` was a descendant of `b`, and both are moved to be direct descendants of
    // `a`. `c` remains a descendant of `b`.
    let output = work_dir.run_jj(["rebase", "-s=b", "-s=d", "-d=a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Working copy  (@) now at: vruxwmqv f1e71cb7 d | d
    Parent commit (@-)      : rlvkpnrz 2443ea76 a | a
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  d: a
    │ ○  c: a b
    ╭─┤
    │ ○  b: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");

    // Same test as above, but with multiple commits per argument
    work_dir.run_jj(["undo"]).success();
    let output = work_dir.run_jj(["rebase", "-s=b|d", "-d=a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Revset `b|d` resolved to more than one revision
    Hint: The revset `b|d` resolved to these revisions:
      vruxwmqv df54a9fd d | d
      zsuskuln d370aee1 b | b
    Hint: Prefix the expression with `all:` to allow any number of revisions (i.e. `all:b|d`).
    [EOF]
    [exit status: 1]
    ");
    let output = work_dir.run_jj(["rebase", "-s=all:b|d", "-d=a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Working copy  (@) now at: vruxwmqv d17539f7 d | d
    Parent commit (@-)      : rlvkpnrz 2443ea76 a | a
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  d: a
    │ ○  c: a b
    ╭─┤
    │ ○  b: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
}

#[test]
fn test_rebase_error_revision_does_not_exist() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    work_dir.run_jj(["describe", "-m", "one"]).success();
    work_dir
        .run_jj(["bookmark", "create", "-r@", "b-one"])
        .success();
    work_dir.run_jj(["new", "-r", "@-", "-m", "two"]).success();

    let output = work_dir.run_jj(["rebase", "-b", "b-one", "-d", "this"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Revision `this` doesn't exist
    [EOF]
    [exit status: 1]
    ");

    let output = work_dir.run_jj(["rebase", "-b", "this", "-d", "b-one"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Revision `this` doesn't exist
    [EOF]
    [exit status: 1]
    ");
}

// This behavior illustrates https://github.com/jj-vcs/jj/issues/2600
#[test]
fn test_rebase_with_child_and_descendant_bug_2600() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "notroot", &[]);
    create_commit(&work_dir, "base", &["notroot"]);
    create_commit(&work_dir, "a", &["base"]);
    create_commit(&work_dir, "b", &["base", "a"]);
    create_commit(&work_dir, "c", &["b"]);
    let setup_opid = work_dir.current_operation_id();

    // Test the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: b
    ○    b: base a
    ├─╮
    │ ○  a: base
    ├─╯
    ○  base: notroot
    ○  notroot
    ◆
    [EOF]
    ");

    // ===================== rebase -s tests =================
    // This should be a no-op
    let output = work_dir.run_jj(["rebase", "-s", "base", "-d", "notroot"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 4 commits that were already in place
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: b
    ○    b: base a
    ├─╮
    │ ○  a: base
    ├─╯
    ○  base: notroot
    ○  notroot
    ◆
    [EOF]
    ");

    work_dir.run_jj(["op", "restore", &setup_opid]).success();
    // This should be a no-op
    let output = work_dir.run_jj(["rebase", "-s", "a", "-d", "base"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 3 commits that were already in place
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: b
    ○    b: base a
    ├─╮
    │ ○  a: base
    ├─╯
    ○  base: notroot
    ○  notroot
    ◆
    [EOF]
    ");

    work_dir.run_jj(["op", "restore", &setup_opid]).success();
    let output = work_dir.run_jj(["rebase", "-s", "a", "-d", "root()"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Working copy  (@) now at: znkkpsqq cf8ecff5 c | c
    Parent commit (@-)      : vruxwmqv 24e1a270 b | b
    [EOF]
    ");
    // Commit "a" should be rebased onto the root commit. Commit "b" should have
    // "base" and "a" as parents as before.
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: b
    ○    b: base a
    ├─╮
    │ ○  a
    ○ │  base: notroot
    ○ │  notroot
    ├─╯
    ◆
    [EOF]
    ");

    // ===================== rebase -b tests =================
    // ====== Reminder of the setup =========
    work_dir.run_jj(["op", "restore", &setup_opid]).success();
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: b
    ○    b: base a
    ├─╮
    │ ○  a: base
    ├─╯
    ○  base: notroot
    ○  notroot
    ◆
    [EOF]
    ");

    // The commits in roots(base..c), i.e. commit "a" should be rebased onto "base",
    // which is a no-op
    let output = work_dir.run_jj(["rebase", "-b", "c", "-d", "base"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 3 commits that were already in place
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: b
    ○    b: base a
    ├─╮
    │ ○  a: base
    ├─╯
    ○  base: notroot
    ○  notroot
    ◆
    [EOF]
    ");

    work_dir.run_jj(["op", "restore", &setup_opid]).success();
    let output = work_dir.run_jj(["rebase", "-b", "c", "-d", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 commits onto destination
    Working copy  (@) now at: znkkpsqq 76914dcc c | c
    Parent commit (@-)      : vruxwmqv f73f03c7 b | b
    [EOF]
    ");
    // The commits in roots(a..c), i.e. commit "b" should be rebased onto "a",
    // which means "b" loses its "base" parent
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: b
    ○  b: a
    ○  a: base
    ○  base: notroot
    ○  notroot
    ◆
    [EOF]
    ");

    work_dir.run_jj(["op", "restore", &setup_opid]).success();
    // This should be a no-op
    let output = work_dir.run_jj(["rebase", "-b", "a", "-d", "root()"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 5 commits that were already in place
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: b
    ○    b: base a
    ├─╮
    │ ○  a: base
    ├─╯
    ○  base: notroot
    ○  notroot
    ◆
    [EOF]
    ");

    // ===================== rebase -r tests =================
    // ====== Reminder of the setup =========
    work_dir.run_jj(["op", "restore", &setup_opid]).success();
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: b
    ○    b: base a
    ├─╮
    │ ○  a: base
    ├─╯
    ○  base: notroot
    ○  notroot
    ◆
    [EOF]
    ");

    let output = work_dir.run_jj(["rebase", "-r", "base", "-d", "root()"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 3 descendant commits
    Working copy  (@) now at: znkkpsqq 45371aaf c | c
    Parent commit (@-)      : vruxwmqv c0a76bf4 b | b
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    // The user would expect unsimplified ancestry here.
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: b
    ○    b: notroot a
    ├─╮
    │ ○  a: notroot
    ├─╯
    ○  notroot
    │ ○  base
    ├─╯
    ◆
    [EOF]
    ");

    // This tests the algorithm for rebasing onto descendants. The result should
    // have unsimplified ancestry.
    work_dir.run_jj(["op", "restore", &setup_opid]).success();
    let output = work_dir.run_jj(["rebase", "-r", "base", "-d", "b"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 3 descendant commits
    Working copy  (@) now at: znkkpsqq e28fa972 c | c
    Parent commit (@-)      : vruxwmqv 8d0eeb6a b | b
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: b
    │ ○  base: b
    ├─╯
    ○    b: notroot a
    ├─╮
    │ ○  a: notroot
    ├─╯
    ○  notroot
    ◆
    [EOF]
    ");

    // This tests the algorithm for rebasing onto descendants. The result should
    // have unsimplified ancestry.
    work_dir.run_jj(["op", "restore", &setup_opid]).success();
    let output = work_dir.run_jj(["rebase", "-r", "base", "-d", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 3 descendant commits
    Working copy  (@) now at: znkkpsqq a9da974c c | c
    Parent commit (@-)      : vruxwmqv 0072139c b | b
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: b
    ○    b: notroot a
    ├─╮
    │ │ ○  base: a
    │ ├─╯
    │ ○  a: notroot
    ├─╯
    ○  notroot
    ◆
    [EOF]
    ");

    work_dir.run_jj(["op", "restore", &setup_opid]).success();
    // ====== Reminder of the setup =========
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: b
    ○    b: base a
    ├─╮
    │ ○  a: base
    ├─╯
    ○  base: notroot
    ○  notroot
    ◆
    [EOF]
    ");

    let output = work_dir.run_jj(["rebase", "-r", "a", "-d", "root()"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 2 descendant commits
    Working copy  (@) now at: znkkpsqq 7210b05e c | c
    Parent commit (@-)      : vruxwmqv da3f7511 b | b
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    // In this case, it is unclear whether the user would always prefer unsimplified
    // ancestry (whether `b` should also be a direct child of the root commit).
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: b
    ○  b: base
    ○  base: notroot
    ○  notroot
    │ ○  a
    ├─╯
    ◆
    [EOF]
    ");

    work_dir.run_jj(["op", "restore", &setup_opid]).success();
    let output = work_dir.run_jj(["rebase", "-r", "b", "-d", "root()"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 1 descendant commits
    Working copy  (@) now at: znkkpsqq f280545e c | c
    Parent commit (@-)      : zsuskuln 0a7fb8f6 base | base
    Parent commit (@-)      : royxmykx 86a06598 a | a
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    // The user would expect unsimplified ancestry here.
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    c: base a
    ├─╮
    │ ○  a: base
    ├─╯
    ○  base: notroot
    ○  notroot
    │ ○  b
    ├─╯
    ◆
    [EOF]
    ");

    // This tests the algorithm for rebasing onto descendants. The result should
    // have unsimplified ancestry.
    work_dir.run_jj(["op", "restore", &setup_opid]).success();
    let output = work_dir.run_jj(["rebase", "-r", "b", "-d", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 1 descendant commits
    Working copy  (@) now at: znkkpsqq c0a7cd80 c | c
    Parent commit (@-)      : zsuskuln 0a7fb8f6 base | base
    Parent commit (@-)      : royxmykx 86a06598 a | a
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○  b: c
    @    c: base a
    ├─╮
    │ ○  a: base
    ├─╯
    ○  base: notroot
    ○  notroot
    ◆
    [EOF]
    ");

    // In this test, the commit with weird ancestry is not rebased (neither directly
    // nor indirectly).
    work_dir.run_jj(["op", "restore", &setup_opid]).success();
    let output = work_dir.run_jj(["rebase", "-r", "c", "-d", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Working copy  (@) now at: znkkpsqq 7a3bc050 c | c
    Parent commit (@-)      : royxmykx 86a06598 a | a
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  c: a
    │ ○  b: base a
    ╭─┤
    ○ │  a: base
    ├─╯
    ○  base: notroot
    ○  notroot
    ◆
    [EOF]
    ");
}

#[test]
fn test_rebase_after() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b1", &["a"]);
    create_commit(&work_dir, "b2", &["b1"]);
    create_commit(&work_dir, "b3", &["a"]);
    create_commit(&work_dir, "b4", &["b3"]);
    create_commit(&work_dir, "c", &["b2", "b4"]);
    create_commit(&work_dir, "d", &["c"]);
    create_commit(&work_dir, "e", &["c"]);
    create_commit(&work_dir, "f", &["e"]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    let setup_opid = work_dir.current_operation_id();

    // Rebasing a commit after its parents should be a no-op.
    let output = work_dir.run_jj(["rebase", "-r", "c", "--after", "b2", "--after", "b4"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 4 commits that were already in place
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");

    // Rebasing a commit after itself should be a no-op.
    let output = work_dir.run_jj(["rebase", "-r", "c", "--after", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 4 commits that were already in place
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");

    // Rebase a commit after another commit. "c" has parents "b2" and "b4", so its
    // children "d" and "e" should be rebased onto "b2" and "b4" respectively.
    let output = work_dir.run_jj(["rebase", "-r", "c", "--after", "e"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 3 descendant commits
    Working copy  (@) now at: xznxytkn e0e873c8 f | f
    Parent commit (@-)      : kmkuslsw 754793f3 c | c
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: c
    ○  c: e
    ○    e: b2 b4
    ├─╮
    │ │ ○  d: b2 b4
    ╭─┬─╯
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a commit after a leaf commit.
    let output = work_dir.run_jj(["rebase", "-r", "e", "--after", "f"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 1 descendant commits
    Working copy  (@) now at: xznxytkn 9804b742 f | f
    Parent commit (@-)      : kmkuslsw cd86b3e4 c | c
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○  e: f
    @  f: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a commit after a commit in a bookmark of a merge commit.
    let output = work_dir.run_jj(["rebase", "-r", "f", "--after", "b1"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 4 descendant commits
    Working copy  (@) now at: xznxytkn 80c27408 f | f
    Parent commit (@-)      : zsuskuln 072d5ae1 b1 | b1
    Added 0 files, modified 0 files, removed 5 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: f
    @ │  f: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a commit after the last commit in a bookmark of a merge commit.
    let output = work_dir.run_jj(["rebase", "-r", "f", "--after", "b2"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 3 descendant commits
    Working copy  (@) now at: xznxytkn ebbc24b1 f | f
    Parent commit (@-)      : royxmykx 2b8e1148 b2 | b2
    Added 0 files, modified 0 files, removed 4 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: f b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    @ │  f: b2
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a commit after a commit with multiple children.
    // "c" has two children "d" and "e", so the rebased commit "f" will inherit the
    // two children.
    let output = work_dir.run_jj(["rebase", "-r", "f", "--after", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 2 descendant commits
    Working copy  (@) now at: xznxytkn 8f8c91d3 f | f
    Parent commit (@-)      : kmkuslsw cd86b3e4 c | c
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○  e: f
    │ ○  d: f
    ├─╯
    @  f: c
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a commit after multiple commits.
    let output = work_dir.run_jj(["rebase", "-r", "f", "--after", "e", "--after", "d"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Working copy  (@) now at: xznxytkn 7784e5a0 f | f
    Parent commit (@-)      : nkmrtpmo 858693f7 e | e
    Parent commit (@-)      : lylxulpl 7d0512e5 d | d
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    f: e d
    ├─╮
    │ ○  d: c
    ○ │  e: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase two unrelated commits.
    let output = work_dir.run_jj(["rebase", "-r", "d", "-r", "e", "--after", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 commits onto destination
    Rebased 6 descendant commits
    Working copy  (@) now at: xznxytkn 0b53613e f | f
    Parent commit (@-)      : kmkuslsw 193687bb c | c
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: c
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○    b3: d e
    │ ├─╮
    ○ │ │  b2: b1
    ○ │ │  b1: d e
    ╰─┬─╮
      │ ○  e: a
      ○ │  d: a
      ├─╯
      ○  a
      ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a subgraph with merge commit and two parents, which should preserve
    // the merge.
    let output = work_dir.run_jj(["rebase", "-r", "b2", "-r", "b4", "-r", "c", "--after", "f"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Rebased 3 descendant commits
    Working copy  (@) now at: xznxytkn eaf1d6b8 f | f
    Parent commit (@-)      : nkmrtpmo 0d7e4ce9 e | e
    Added 0 files, modified 0 files, removed 3 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○    c: b2 b4
    ├─╮
    │ ○  b4: f
    ○ │  b2: f
    ├─╯
    @  f: e
    ○    e: b1 b3
    ├─╮
    │ │ ○  d: b1 b3
    ╭─┬─╯
    │ ○  b3: a
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a subgraph with four commits after one of the commits itself.
    let output = work_dir.run_jj(["rebase", "-r", "b1::d", "--after", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 4 commits onto destination
    Rebased 2 descendant commits
    Working copy  (@) now at: xznxytkn 9bc7e54c f | f
    Parent commit (@-)      : nkmrtpmo 0f80251b e | e
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○  e: d
    ○  d: c
    ○    c: b2 b4
    ├─╮
    ○ │  b2: b1
    ○ │  b1: a b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a subgraph before the parents of one of the commits in the subgraph.
    // "c" had parents "b2" and "b4", but no longer has "b4" as a parent since
    // "b4" would be a descendant of "c" after the rebase.
    let output = work_dir.run_jj(["rebase", "-r", "b2::d", "--after", "root()"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Rebased 6 descendant commits
    Working copy  (@) now at: xznxytkn 0875aabc f | f
    Parent commit (@-)      : nkmrtpmo d429661b e | e
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○    e: b1 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b1: a
    ├─╯
    ○  a: d
    ○  d: c
    ○  c: b2
    ○  b2
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a subgraph with disconnected commits. Since "b2" is an ancestor of
    // "e", "b2" should be a parent of "e" after the rebase.
    let output = work_dir.run_jj(["rebase", "-r", "e", "-r", "b2", "--after", "d"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 commits onto destination
    Rebased 3 descendant commits
    Working copy  (@) now at: xznxytkn 3238a418 f | f
    Parent commit (@-)      : kmkuslsw 6a51bd41 c | c
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: c
    │ ○  e: b2
    │ ○  b2: d
    │ ○  d: c
    ├─╯
    ○    c: b1 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // `rebase -s` of commit "c" and its descendants after itself should be a no-op.
    let output = work_dir.run_jj(["rebase", "-s", "c", "--after", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 4 commits that were already in place
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // `rebase -s` of a commit and its descendants after multiple commits.
    let output = work_dir.run_jj(["rebase", "-s", "c", "--after", "b1", "--after", "b3"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 4 commits onto destination
    Rebased 2 descendant commits
    Working copy  (@) now at: xznxytkn a4ace41c f | f
    Parent commit (@-)      : nkmrtpmo c7744d08 e | e
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○    b4: d f
    ├─╮
    │ │ ○  b2: d f
    ╭─┬─╯
    │ @  f: e
    │ ○  e: c
    ○ │  d: c
    ├─╯
    ○    c: b1 b3
    ├─╮
    │ ○  b3: a
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // `rebase -b` of commit "b3" after "b1" moves its descendants which are not
    // already descendants of "b1" (just "b3" and "b4") in between "b1" and its
    // child "b2".
    let output = work_dir.run_jj(["rebase", "-b", "b3", "--after", "b1"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 6 commits onto destination
    Rebased 1 descendant commits
    Working copy  (@) now at: xznxytkn b4078b57 f | f
    Parent commit (@-)      : nkmrtpmo 1b95558f e | e
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○    b2: d f
    ├─╮
    │ @  f: e
    │ ○  e: c
    ○ │  d: c
    ├─╯
    ○  c: b4
    ○  b4: b3
    ○  b3: b1
    ○  b1: a
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Should error if a loop will be created.
    let output = work_dir.run_jj(["rebase", "-r", "e", "--after", "a", "--after", "b2"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Refusing to create a loop: commit 2b8e1148290f would be both an ancestor and a descendant of the rebased commits
    [EOF]
    [exit status: 1]
    ");
}

#[test]
fn test_rebase_before() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b1", &["a"]);
    create_commit(&work_dir, "b2", &["b1"]);
    create_commit(&work_dir, "b3", &["a"]);
    create_commit(&work_dir, "b4", &["b3"]);
    create_commit(&work_dir, "c", &["b2", "b4"]);
    create_commit(&work_dir, "d", &["c"]);
    create_commit(&work_dir, "e", &["c"]);
    create_commit(&work_dir, "f", &["e"]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    let setup_opid = work_dir.current_operation_id();

    // Rebasing a commit before its children should be a no-op.
    let output = work_dir.run_jj(["rebase", "-r", "c", "--before", "d", "--before", "e"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 4 commits that were already in place
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");

    // Rebasing a commit before itself should be a no-op.
    let output = work_dir.run_jj(["rebase", "-r", "c", "--before", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 4 commits that were already in place
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");

    // Rebasing a commit before the root commit should error.
    let output = work_dir.run_jj(["rebase", "-r", "c", "--before", "root()"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: The root commit 000000000000 is immutable
    [EOF]
    [exit status: 1]
    ");

    // Rebase a commit before another commit. "c" has parents "b2" and "b4", so its
    // children "d" and "e" should be rebased onto "b2" and "b4" respectively.
    let output = work_dir.run_jj(["rebase", "-r", "c", "--before", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 8 descendant commits
    Working copy  (@) now at: xznxytkn 24335685 f | f
    Parent commit (@-)      : nkmrtpmo e9a28d4b e | e
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○    e: b2 b4
    ├─╮
    │ │ ○  d: b2 b4
    ╭─┬─╯
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a: c
    ○  c
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a commit before its parent.
    let output = work_dir.run_jj(["rebase", "-r", "f", "--before", "e"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 1 descendant commits
    Working copy  (@) now at: xznxytkn 8e3b728a f | f
    Parent commit (@-)      : kmkuslsw cd86b3e4 c | c
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○  e: f
    @  f: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a commit before a commit in a bookmark of a merge commit.
    let output = work_dir.run_jj(["rebase", "-r", "f", "--before", "b2"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 4 descendant commits
    Working copy  (@) now at: xznxytkn 2b4f48f8 f | f
    Parent commit (@-)      : zsuskuln 072d5ae1 b1 | b1
    Added 0 files, modified 0 files, removed 5 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: f
    @ │  f: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a commit before the first commit in a bookmark of a merge commit.
    let output = work_dir.run_jj(["rebase", "-r", "f", "--before", "b1"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 5 descendant commits
    Working copy  (@) now at: xznxytkn 488ebb95 f | f
    Parent commit (@-)      : rlvkpnrz 2443ea76 a | a
    Added 0 files, modified 0 files, removed 6 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: f
    @ │  f: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a commit before a merge commit. "c" has two parents "b2" and "b4", so
    // the rebased commit "f" will have the two commits "b2" and "b4" as its
    // parents.
    let output = work_dir.run_jj(["rebase", "-r", "f", "--before", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 3 descendant commits
    Working copy  (@) now at: xznxytkn aae1bc10 f | f
    Parent commit (@-)      : royxmykx 2b8e1148 b2 | b2
    Parent commit (@-)      : znkkpsqq a52a83a4 b4 | b4
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○  e: c
    │ ○  d: c
    ├─╯
    ○  c: f
    @    f: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a commit before multiple commits.
    let output = work_dir.run_jj(["rebase", "-r", "b1", "--before", "d", "--before", "e"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 5 descendant commits
    Working copy  (@) now at: xznxytkn 8268ec4d f | f
    Parent commit (@-)      : nkmrtpmo fd26fbd4 e | e
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○  e: b1
    │ ○  d: b1
    ├─╯
    ○  b1: c
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a commit before two commits in separate bookmarks to create a merge
    // commit.
    let output = work_dir.run_jj(["rebase", "-r", "f", "--before", "b2", "--before", "b4"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 5 descendant commits
    Working copy  (@) now at: xznxytkn 7ba8014f f | f
    Parent commit (@-)      : zsuskuln 072d5ae1 b1 | b1
    Parent commit (@-)      : vruxwmqv 523e6a8b b3 | b3
    Added 0 files, modified 0 files, removed 4 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: f
    ○ │  b2: f
    ├─╯
    @    f: b1 b3
    ├─╮
    │ ○  b3: a
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase two unrelated commits "b2" and "b4" before a single commit "a". This
    // creates a merge commit "a" with the two parents "b2" and "b4".
    let output = work_dir.run_jj(["rebase", "-r", "b2", "-r", "b4", "--before", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 commits onto destination
    Rebased 7 descendant commits
    Working copy  (@) now at: xznxytkn fabd8dd7 f | f
    Parent commit (@-)      : nkmrtpmo b5933877 e | e
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: b1 b3
    ├─╮
    │ ○  b3: a
    ○ │  b1: a
    ├─╯
    ○    a: b2 b4
    ├─╮
    │ ○  b4
    ○ │  b2
    ├─╯
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a subgraph with a merge commit and two parents.
    let output = work_dir.run_jj(["rebase", "-r", "b2", "-r", "b4", "-r", "c", "--before", "e"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Rebased 3 descendant commits
    Working copy  (@) now at: xznxytkn cbe2be58 f | f
    Parent commit (@-)      : nkmrtpmo e31053d1 e | e
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○  e: c
    ○    c: b2 b4
    ├─╮
    │ ○    b4: b1 b3
    │ ├─╮
    ○ │ │  b2: b1 b3
    ╰─┬─╮
    ○ │ │  d: b1 b3
    ╰─┬─╮
      │ ○  b3: a
      ○ │  b1: a
      ├─╯
      ○  a
      ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a subgraph with disconnected commits. Since "b1" is an ancestor of
    // "e", "b1" should be a parent of "e" after the rebase.
    let output = work_dir.run_jj(["rebase", "-r", "b1", "-r", "e", "--before", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 commits onto destination
    Rebased 7 descendant commits
    Working copy  (@) now at: xznxytkn 1c48b514 f | f
    Parent commit (@-)      : kmkuslsw c0fd979a c | c
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: a
    ├─╯
    ○  a: e
    ○  e: b1
    ○  b1
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a subgraph before the parents of one of the commits in the subgraph.
    // "c" had parents "b2" and "b4", but no longer has "b4" as a parent since
    // "b4" would be a descendant of "c" after the rebase.
    let output = work_dir.run_jj(["rebase", "-r", "b2::d", "--before", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Rebased 6 descendant commits
    Working copy  (@) now at: xznxytkn f5991dc7 f | f
    Parent commit (@-)      : nkmrtpmo 37894e3c e | e
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○    e: b1 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b1: a
    ├─╯
    ○  a: d
    ○  d: c
    ○  c: b2
    ○  b2
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a subgraph before the parents of one of the commits in the subgraph.
    // "c" had parents "b2" and "b4", but no longer has "b4" as a parent since
    // "b4" would be a descendant of "c" after the rebase.
    let output = work_dir.run_jj(["rebase", "-r", "b2::d", "--before", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Rebased 6 descendant commits
    Working copy  (@) now at: xznxytkn 308a31e9 f | f
    Parent commit (@-)      : nkmrtpmo 538444a5 e | e
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○    e: b1 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b1: a
    ├─╯
    ○  a: d
    ○  d: c
    ○  c: b2
    ○  b2
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // `rebase -s` of commit "c" and its descendants before itself should be a
    // no-op.
    let output = work_dir.run_jj(["rebase", "-s", "c", "--before", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 4 commits that were already in place
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: b2 b4
    ├─╮
    │ ○  b4: b3
    │ ○  b3: a
    ○ │  b2: b1
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // `rebase -s` of a commit and its descendants before multiple commits.
    let output = work_dir.run_jj(["rebase", "-s", "c", "--before", "b2", "--before", "b4"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 4 commits onto destination
    Rebased 2 descendant commits
    Working copy  (@) now at: xznxytkn 84704387 f | f
    Parent commit (@-)      : nkmrtpmo cff61821 e | e
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○    b4: d f
    ├─╮
    │ │ ○  b2: d f
    ╭─┬─╯
    │ @  f: e
    │ ○  e: c
    ○ │  d: c
    ├─╯
    ○    c: b1 b3
    ├─╮
    │ ○  b3: a
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // `rebase -b` of commit "b3" before "b2" moves its descendants which are not
    // already descendants of its parent "b1" (just "b3" and "b4") in between "b1"
    // and its child "b2".
    let output = work_dir.run_jj(["rebase", "-b", "b3", "--before", "b1"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 2 commits that were already in place
    Rebased 4 commits onto destination
    Rebased 2 descendant commits
    Working copy  (@) now at: xznxytkn 16422f85 f | f
    Parent commit (@-)      : nkmrtpmo ef9dea83 e | e
    Added 0 files, modified 0 files, removed 2 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○  b2: b1
    ○    b1: d f
    ├─╮
    │ @  f: e
    │ ○  e: c
    ○ │  d: c
    ├─╯
    ○  c: b4
    ○  b4: b3
    ○  b3: a
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Should error if a loop will be created.
    let output = work_dir.run_jj(["rebase", "-r", "e", "--before", "b2", "--before", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Refusing to create a loop: commit 2b8e1148290f would be both an ancestor and a descendant of the rebased commits
    [EOF]
    [exit status: 1]
    ");
}

#[test]
fn test_rebase_after_before() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "x", &[]);
    create_commit(&work_dir, "y", &["x"]);
    create_commit(&work_dir, "z", &["y"]);
    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b1", &["a"]);
    create_commit(&work_dir, "b2", &["a"]);
    create_commit(&work_dir, "c", &["b1", "b2"]);
    create_commit(&work_dir, "d", &["c"]);
    create_commit(&work_dir, "e", &["c"]);
    create_commit(&work_dir, "f", &["e"]);
    // Test the setup
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○  e: c
    │ ○  d: c
    ├─╯
    ○    c: b1 b2
    ├─╮
    │ ○  b2: a
    ○ │  b1: a
    ├─╯
    ○  a
    │ ○  z: y
    │ ○  y: x
    │ ○  x
    ├─╯
    ◆
    [EOF]
    ");
    let setup_opid = work_dir.current_operation_id();

    // Rebase a commit after another commit and before that commit's child to
    // insert directly between the two commits.
    let output = work_dir.run_jj(["rebase", "-r", "d", "--after", "e", "--before", "f"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 1 descendant commits
    Working copy  (@) now at: nmzmmopx 56c81c6d f | f
    Parent commit (@-)      : nkmrtpmo ff196f69 d | d
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: d
    ○  d: e
    ○  e: c
    ○    c: b1 b2
    ├─╮
    │ ○  b2: a
    ○ │  b1: a
    ├─╯
    ○  a
    │ ○  z: y
    │ ○  y: x
    │ ○  x
    ├─╯
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase a commit after another commit and before that commit's descendant to
    // create a new merge commit.
    let output = work_dir.run_jj(["rebase", "-r", "d", "--after", "a", "--before", "f"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 1 descendant commits
    Working copy  (@) now at: nmzmmopx 398173ed f | f
    Parent commit (@-)      : xznxytkn b3e6aadf e | e
    Parent commit (@-)      : nkmrtpmo db529447 d | d
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @    f: e d
    ├─╮
    │ ○  d: a
    ○ │  e: c
    ○ │    c: b1 b2
    ├───╮
    │ │ ○  b2: a
    │ ├─╯
    ○ │  b1: a
    ├─╯
    ○  a
    │ ○  z: y
    │ ○  y: x
    │ ○  x
    ├─╯
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // "c" has parents "b1" and "b2", so when it is rebased, its children "d" and
    // "e" should have "b1" and "b2" as parents as well. "c" is then inserted in
    // between "d" and "e", making "e" a merge commit with 3 parents "b1", "b2",
    // and "c".
    let output = work_dir.run_jj(["rebase", "-r", "c", "--after", "d", "--before", "e"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 1 commits onto destination
    Rebased 3 descendant commits
    Working copy  (@) now at: nmzmmopx 2be98daf f | f
    Parent commit (@-)      : xznxytkn 911fc846 e | e
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○      e: b1 b2 c
    ├─┬─╮
    │ │ ○  c: d
    │ │ ○  d: b1 b2
    ╭─┬─╯
    │ ○  b2: a
    ○ │  b1: a
    ├─╯
    ○  a
    │ ○  z: y
    │ ○  y: x
    │ ○  x
    ├─╯
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Rebase multiple commits and preserve their ancestry. Apart from the heads of
    // the target commits ("d" and "e"), "f" also has commits "b1" and "b2" as
    // parents since its parents "d" and "e" were in the target set and were
    // replaced by their closest ancestors outside the target set.
    let output = work_dir.run_jj([
        "rebase", "-r", "c", "-r", "d", "-r", "e", "--after", "a", "--before", "f",
    ]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Rebased 1 descendant commits
    Working copy  (@) now at: nmzmmopx bee09b10 f | f
    Parent commit (@-)      : znkkpsqq 9167144b b1 | b1
    Parent commit (@-)      : kmkuslsw 87fed139 b2 | b2
    Parent commit (@-)      : nkmrtpmo 4a8ca156 d | d
    Parent commit (@-)      : xznxytkn 0cc1825e e | e
    Added 1 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @        f: b1 b2 d e
    ├─┬─┬─╮
    │ │ │ ○  e: c
    │ │ ○ │  d: c
    │ │ ├─╯
    │ │ ○  c: a
    │ ○ │  b2: a
    │ ├─╯
    ○ │  b1: a
    ├─╯
    ○  a
    │ ○  z: y
    │ ○  y: x
    │ ○  x
    ├─╯
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // `rebase -s` of a commit and its descendants.
    let output = work_dir.run_jj(["rebase", "-s", "c", "--before", "b1", "--after", "b2"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 4 commits onto destination
    Rebased 1 descendant commits
    Working copy  (@) now at: nmzmmopx 951204cf f | f
    Parent commit (@-)      : xznxytkn fe8ec4e2 e | e
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    ○      b1: a d f
    ├─┬─╮
    │ │ @  f: e
    │ │ ○  e: c
    │ ○ │  d: c
    │ ├─╯
    │ ○  c: b2
    │ ○  b2: a
    ├─╯
    ○  a
    │ ○  z: y
    │ ○  y: x
    │ ○  x
    ├─╯
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // `rebase -b` of a commit "y" to a destination after "a" will rebase all
    // commits in "roots(a..y)" and their descendants, corresponding to "x", "y"
    // and "z". They will be inserted in a new branch after "a" and before "c".
    let output = work_dir.run_jj(["rebase", "-b", "y", "--after", "a", "--before", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 3 commits onto destination
    Rebased 4 descendant commits
    Working copy  (@) now at: nmzmmopx 4496f88e f | f
    Parent commit (@-)      : xznxytkn a85404a6 e | e
    Added 3 files, modified 0 files, removed 0 files
    [EOF]
    ");
    insta::assert_snapshot!(get_log_output(&work_dir), @r"
    @  f: e
    ○  e: c
    │ ○  d: c
    ├─╯
    ○      c: b1 b2 z
    ├─┬─╮
    │ │ ○  z: y
    │ │ ○  y: x
    │ │ ○  x: a
    │ ○ │  b2: a
    │ ├─╯
    ○ │  b1: a
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
    work_dir.run_jj(["op", "restore", &setup_opid]).success();

    // Should error if a loop will be created.
    let output = work_dir.run_jj(["rebase", "-r", "e", "--after", "c", "--before", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Error: Refusing to create a loop: commit 31b84afe1c8f would be both an ancestor and a descendant of the rebased commits
    [EOF]
    [exit status: 1]
    ");
}

#[test]
fn test_rebase_skip_emptied() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b", &["a"]);
    work_dir
        .run_jj(["new", "a", "-m", "will become empty"])
        .success();
    work_dir.run_jj(["restore", "--from=b"]).success();
    work_dir.run_jj(["new", "-m", "already empty"]).success();
    work_dir
        .run_jj(["new", "-m", "also already empty"])
        .success();
    let setup_opid = work_dir.current_operation_id();

    // Test the setup
    insta::assert_snapshot!(work_dir.run_jj(["log", "-T", "description"]), @r"
    @  also already empty
    ○  already empty
    ○  will become empty
    │ ○  b
    ├─╯
    ○  a
    ◆
    [EOF]
    ");

    let output = work_dir.run_jj(["rebase", "-d=b", "--skip-emptied"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 commits onto destination
    Abandoned 1 newly emptied commits
    Working copy  (@) now at: yostqsxw bc4222f2 (empty) also already empty
    Parent commit (@-)      : vruxwmqv 6b41ecb2 (empty) already empty
    [EOF]
    ");

    // The parent commit became empty and was dropped, but the already empty commits
    // were kept
    insta::assert_snapshot!(work_dir.run_jj(["log", "-T", "description"]), @r"
    @  also already empty
    ○  already empty
    ○  b
    ○  a
    ◆
    [EOF]
    ");

    work_dir.run_jj(["op", "restore", &setup_opid]).success();
    // Test the setup
    insta::assert_snapshot!(work_dir.run_jj(["log", "-T", "description"]), @r"
    @  also already empty
    ○  already empty
    ○  will become empty
    │ ○  b
    ├─╯
    ○  a
    ◆
    [EOF]
    ");

    let output = work_dir.run_jj([
        "rebase",
        "-r=description('will become empty')",
        "-d=b",
        "--skip-emptied",
    ]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Rebased 2 descendant commits
    Abandoned 1 newly emptied commits
    Working copy  (@) now at: yostqsxw 74149b9b (empty) also already empty
    Parent commit (@-)      : vruxwmqv 3bdb2801 (empty) already empty
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");

    // Rebasing a single commit which becomes empty abandons that commit, whilst its
    // already empty descendants were kept
    insta::assert_snapshot!(work_dir.run_jj(["log", "-T", "description"]), @r"
    @  also already empty
    ○  already empty
    │ ○  b
    ├─╯
    ○  a
    ◆
    [EOF]
    ");
}

#[test]
fn test_rebase_skip_emptied_descendants() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b", &["a"]);
    work_dir
        .run_jj(["new", "a", "-m", "c (will become empty)"])
        .success();
    work_dir.run_jj(["restore", "--from=b"]).success();
    work_dir
        .run_jj(["bookmark", "create", "-r@", "c"])
        .success();
    work_dir.run_jj(["new", "-m", "already empty"]).success();
    work_dir
        .run_jj(["new", "-m", "also already empty"])
        .success();

    // Test the setup
    insta::assert_snapshot!(work_dir.run_jj(["log", "-T", "description"]), @r"
    @  also already empty
    ○  already empty
    ○  c (will become empty)
    │ ○  b
    ├─╯
    ○  a
    ◆
    [EOF]
    ");

    let output = work_dir.run_jj(["rebase", "-r", "b", "--before", "c", "--skip-emptied"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 1 commits that were already in place
    Rebased 3 descendant commits
    Working copy  (@) now at: znkkpsqq 353bac5c (empty) also already empty
    Parent commit (@-)      : yostqsxw 0a3f76fd (empty) already empty
    [EOF]
    ");

    // Commits not in the rebase target set should not be abandoned even if they
    // were emptied.
    insta::assert_snapshot!(work_dir.run_jj(["log", "-T", "description"]), @r"
    @  also already empty
    ○  already empty
    ○  c (will become empty)
    ○  b
    ○  a
    ◆
    [EOF]
    ");
}

#[test]
fn test_rebase_skip_if_on_destination() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    create_commit(&work_dir, "a", &[]);
    create_commit(&work_dir, "b1", &["a"]);
    create_commit(&work_dir, "b2", &["a"]);
    create_commit(&work_dir, "c", &["b1", "b2"]);
    create_commit(&work_dir, "d", &["c"]);
    create_commit(&work_dir, "e", &["c"]);
    create_commit(&work_dir, "f", &["e"]);
    // Test the setup
    insta::assert_snapshot!(get_long_log_output(&work_dir), @r"
    @  f  lylxulpl  88f778c5:  e
    ○  e  kmkuslsw  48dd9e3f:  c
    │ ○  d  znkkpsqq  92438fc9:  c
    ├─╯
    ○    c  vruxwmqv  c41e416e:  b1 b2
    ├─╮
    │ ○  b2  royxmykx  903ab0d6:  a
    ○ │  b1  zsuskuln  072d5ae1:  a
    ├─╯
    ○  a  rlvkpnrz  2443ea76
    ◆    zzzzzzzz  00000000
    [EOF]
    ");

    // Skip rebase with -b
    let output = work_dir.run_jj(["rebase", "-b", "d", "-d", "a"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 6 commits that were already in place
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(get_long_log_output(&work_dir), @r"
    @  f  lylxulpl  88f778c5:  e
    ○  e  kmkuslsw  48dd9e3f:  c
    │ ○  d  znkkpsqq  92438fc9:  c
    ├─╯
    ○    c  vruxwmqv  c41e416e:  b1 b2
    ├─╮
    │ ○  b2  royxmykx  903ab0d6:  a
    ○ │  b1  zsuskuln  072d5ae1:  a
    ├─╯
    ○  a  rlvkpnrz  2443ea76
    ◆    zzzzzzzz  00000000
    [EOF]
    ");

    // Skip rebase with -s
    let output = work_dir.run_jj(["rebase", "-s", "c", "-d", "b1", "-d", "b2"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 4 commits that were already in place
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(get_long_log_output(&work_dir), @r"
    @  f  lylxulpl  88f778c5:  e
    ○  e  kmkuslsw  48dd9e3f:  c
    │ ○  d  znkkpsqq  92438fc9:  c
    ├─╯
    ○    c  vruxwmqv  c41e416e:  b1 b2
    ├─╮
    │ ○  b2  royxmykx  903ab0d6:  a
    ○ │  b1  zsuskuln  072d5ae1:  a
    ├─╯
    ○  a  rlvkpnrz  2443ea76
    ◆    zzzzzzzz  00000000
    [EOF]
    ");

    // Skip rebase with -r since commit has no children
    let output = work_dir.run_jj(["rebase", "-r", "d", "-d", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 1 commits that were already in place
    Nothing changed.
    [EOF]
    ");
    insta::assert_snapshot!(get_long_log_output(&work_dir), @r"
    @  f  lylxulpl  88f778c5:  e
    ○  e  kmkuslsw  48dd9e3f:  c
    │ ○  d  znkkpsqq  92438fc9:  c
    ├─╯
    ○    c  vruxwmqv  c41e416e:  b1 b2
    ├─╮
    │ ○  b2  royxmykx  903ab0d6:  a
    ○ │  b1  zsuskuln  072d5ae1:  a
    ├─╯
    ○  a  rlvkpnrz  2443ea76
    ◆    zzzzzzzz  00000000
    [EOF]
    ");

    // Skip rebase of commit, but rebases children onto destination with -r
    let output = work_dir.run_jj(["rebase", "-r", "e", "-d", "c"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Skipped rebase of 1 commits that were already in place
    Rebased 1 descendant commits
    Working copy  (@) now at: lylxulpl 77cb229f f | f
    Parent commit (@-)      : vruxwmqv c41e416e c | c
    Added 0 files, modified 0 files, removed 1 files
    [EOF]
    ");
    insta::assert_snapshot!(get_long_log_output(&work_dir), @r"
    @  f  lylxulpl  77cb229f:  c
    │ ○  e  kmkuslsw  48dd9e3f:  c
    ├─╯
    │ ○  d  znkkpsqq  92438fc9:  c
    ├─╯
    ○    c  vruxwmqv  c41e416e:  b1 b2
    ├─╮
    │ ○  b2  royxmykx  903ab0d6:  a
    ○ │  b1  zsuskuln  072d5ae1:  a
    ├─╯
    ○  a  rlvkpnrz  2443ea76
    ◆    zzzzzzzz  00000000
    [EOF]
    ");
}

#[must_use]
fn get_log_output(work_dir: &TestWorkDir) -> CommandOutput {
    let template = "bookmarks ++ surround(': ', '', parents.map(|c| c.bookmarks()))";
    work_dir.run_jj(["log", "-T", template])
}

#[must_use]
fn get_long_log_output(work_dir: &TestWorkDir) -> CommandOutput {
    let template = "bookmarks ++ '  ' ++ change_id.shortest(8) ++ '  ' ++ commit_id.shortest(8) \
                    ++ surround(':  ', '', parents.map(|c| c.bookmarks()))";
    work_dir.run_jj(["log", "-T", template])
}
