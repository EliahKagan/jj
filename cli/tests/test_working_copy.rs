// Copyright 2023 The Jujutsu Authors
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

use indoc::indoc;
use regex::Regex;

use crate::common::TestEnvironment;

#[test]
fn test_snapshot_large_file() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    // test a small file using raw-integer-literal syntax, which is interpreted
    // in bytes
    test_env.add_config(r#"snapshot.max-new-file-size = 10"#);
    work_dir.write_file("empty", "");
    work_dir.write_file("large", "a lot of text");
    let output = work_dir.run_jj(["file", "list"]);
    insta::assert_snapshot!(output, @r"
    empty
    [EOF]
    ------- stderr -------
    Warning: Refused to snapshot some files:
      large: 13.0B (13 bytes); the maximum size allowed is 10.0B (10 bytes)
    Hint: This is to prevent large files from being added by accident. You can fix this by:
      - Adding the file to `.gitignore`
      - Run `jj config set --repo snapshot.max-new-file-size 13`
        This will increase the maximum file size allowed for new files, in this repository only.
      - Run `jj --config snapshot.max-new-file-size=13 st`
        This will increase the maximum file size allowed for new files, for this command only.
    [EOF]
    ");

    // test with a larger file using 'KB' human-readable syntax
    test_env.add_config(r#"snapshot.max-new-file-size = "10KB""#);
    let big_string = vec![0; 1024 * 11];
    work_dir.write_file("large", &big_string);
    let output = work_dir.run_jj(["file", "list"]);
    insta::assert_snapshot!(output, @r"
    empty
    [EOF]
    ------- stderr -------
    Warning: Refused to snapshot some files:
      large: 11.0KiB (11264 bytes); the maximum size allowed is 10.0KiB (10240 bytes)
    Hint: This is to prevent large files from being added by accident. You can fix this by:
      - Adding the file to `.gitignore`
      - Run `jj config set --repo snapshot.max-new-file-size 11264`
        This will increase the maximum file size allowed for new files, in this repository only.
      - Run `jj --config snapshot.max-new-file-size=11264 st`
        This will increase the maximum file size allowed for new files, for this command only.
    [EOF]
    ");

    // test with file track for hint formatting, both files should appear in
    // warnings even though they were snapshotted separately
    work_dir.write_file("large2", big_string);
    let output = work_dir.run_jj([
        "file",
        "--config=snapshot.auto-track='large'",
        "track",
        "large2",
    ]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Warning: Refused to snapshot some files:
      large: 11.0KiB (11264 bytes); the maximum size allowed is 10.0KiB (10240 bytes)
      large2: 11.0KiB (11264 bytes); the maximum size allowed is 10.0KiB (10240 bytes)
    Hint: This is to prevent large files from being added by accident. You can fix this by:
      - Adding the file to `.gitignore`
      - Run `jj config set --repo snapshot.max-new-file-size 11264`
        This will increase the maximum file size allowed for new files, in this repository only.
      - Run `jj --config snapshot.max-new-file-size=11264 file track large large2`
        This will increase the maximum file size allowed for new files, for this command only.
    [EOF]
    ");

    // test invalid configuration
    let output = work_dir.run_jj(["file", "list", "--config=snapshot.max-new-file-size=[]"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Config error: Invalid type or value for snapshot.max-new-file-size
    Caused by: Expected a positive integer or a string in '<number><unit>' form
    For help, see https://jj-vcs.github.io/jj/latest/config/ or use `jj help -k config`.
    [EOF]
    [exit status: 1]
    ");

    // No error if we disable auto-tracking of the path
    let output = work_dir.run_jj(["file", "list", "--config=snapshot.auto-track='none()'"]);
    insta::assert_snapshot!(output, @r"
    empty
    [EOF]
    ");

    // max-new-file-size=0 means no limit
    let output = work_dir.run_jj(["file", "list", "--config=snapshot.max-new-file-size=0"]);
    insta::assert_snapshot!(output, @r"
    empty
    large
    large2
    [EOF]
    ");
}

#[test]
fn test_snapshot_large_file_restore() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");
    test_env.add_config("snapshot.max-new-file-size = 10");

    work_dir.run_jj(["describe", "-mcommitted"]).success();
    work_dir.write_file("file", "small");

    // Write a large file in the working copy, restore it from a commit. The
    // working-copy content shouldn't be overwritten.
    work_dir.run_jj(["new", "root()"]).success();
    work_dir.write_file("file", "a lot of text");
    let output = work_dir.run_jj(["restore", "--from=description(committed)"]);
    insta::assert_snapshot!(output, @r"
    ------- stderr -------
    Warning: Refused to snapshot some files:
      file: 13.0B (13 bytes); the maximum size allowed is 10.0B (10 bytes)
    Hint: This is to prevent large files from being added by accident. You can fix this by:
      - Adding the file to `.gitignore`
      - Run `jj config set --repo snapshot.max-new-file-size 13`
        This will increase the maximum file size allowed for new files, in this repository only.
      - Run `jj --config snapshot.max-new-file-size=13 st`
        This will increase the maximum file size allowed for new files, for this command only.
    Created kkmpptxz e3eb7e81 (no description set)
    Working copy  (@) now at: kkmpptxz e3eb7e81 (no description set)
    Parent commit (@-)      : zzzzzzzz 00000000 (empty) (no description set)
    Added 1 files, modified 0 files, removed 0 files
    Warning: 1 of those updates were skipped because there were conflicting changes in the working copy.
    Hint: Inspect the changes compared to the intended target with `jj diff --from e3eb7e819de5`.
    Discard the conflicting changes with `jj restore --from e3eb7e819de5`.
    [EOF]
    ");
    insta::assert_snapshot!(work_dir.read_file("file"), @"a lot of text");

    // However, the next command will snapshot the large file because it is now
    // tracked. TODO: Should we remember the untracked state?
    let output = work_dir.run_jj(["status"]);
    insta::assert_snapshot!(output, @r"
    Working copy changes:
    A file
    Working copy  (@) : kkmpptxz b75eed09 (no description set)
    Parent commit (@-): zzzzzzzz 00000000 (empty) (no description set)
    [EOF]
    ");
}

#[test]
fn test_materialize_and_snapshot_different_conflict_markers() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    // Configure to use Git-style conflict markers
    test_env.add_config(r#"ui.conflict-marker-style = "git""#);

    // Create a conflict in the working copy
    work_dir.write_file(
        "file",
        indoc! {"
            line 1
            line 2
            line 3
        "},
    );
    work_dir.run_jj(["commit", "-m", "base"]).success();
    work_dir.write_file(
        "file",
        indoc! {"
            line 1
            line 2 - a
            line 3
        "},
    );
    work_dir.run_jj(["commit", "-m", "side-a"]).success();
    work_dir
        .run_jj(["new", "description(base)", "-m", "side-b"])
        .success();
    work_dir.write_file(
        "file",
        indoc! {"
            line 1
            line 2 - b
            line 3 - b
        "},
    );
    work_dir
        .run_jj(["new", "description(side-a)", "description(side-b)"])
        .success();

    // File should have Git-style conflict markers
    insta::assert_snapshot!(work_dir.read_file("file"), @r"
    line 1
    <<<<<<< Side #1 (Conflict 1 of 1)
    line 2 - a
    line 3
    ||||||| Base
    line 2
    line 3
    =======
    line 2 - b
    line 3 - b
    >>>>>>> Side #2 (Conflict 1 of 1 ends)
    ");

    // Configure to use JJ-style "snapshot" conflict markers
    test_env.add_config(r#"ui.conflict-marker-style = "snapshot""#);

    // Update the conflict, still using Git-style conflict markers
    work_dir.write_file(
        "file",
        indoc! {"
            line 1
            <<<<<<<
            line 2 - a
            line 3 - a
            |||||||
            line 2
            line 3
            =======
            line 2 - b
            line 3 - b
            >>>>>>>
        "},
    );

    // Git-style markers should be parsed, then rendered with new config
    insta::assert_snapshot!(work_dir.run_jj(["diff", "--git"]), @r"
    diff --git a/file b/file
    --- a/file
    +++ b/file
    @@ -2,7 +2,7 @@
     <<<<<<< Conflict 1 of 1
     +++++++ Contents of side #1
     line 2 - a
    -line 3
    +line 3 - a
     ------- Contents of base
     line 2
     line 3
    [EOF]
    ");
}

#[test]
fn test_snapshot_invalid_ignore_pattern() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    // Test invalid pattern in .gitignore
    work_dir.write_file(".gitignore", " []\n");
    insta::assert_snapshot!(work_dir.run_jj(["st"]), @r"
    ------- stderr -------
    Internal error: Failed to snapshot the working copy
    Caused by:
    1: Failed to parse ignore patterns from file $TEST_ENV/repo/.gitignore
    2: error parsing glob ' []': unclosed character class; missing ']'
    [EOF]
    [exit status: 255]
    ");

    // Test invalid UTF-8 in .gitignore
    work_dir.write_file(".gitignore", b"\xff\n");
    insta::assert_snapshot!(work_dir.run_jj(["st"]), @r"
    ------- stderr -------
    Internal error: Failed to snapshot the working copy
    Caused by:
    1: Invalid UTF-8 for ignore pattern in $TEST_ENV/repo/.gitignore on line #1: �
    2: invalid utf-8 sequence of 1 bytes from index 0
    [EOF]
    [exit status: 255]
    ");
}

#[test]
fn test_conflict_marker_length_stored_in_working_copy() {
    let test_env = TestEnvironment::default();
    test_env.run_jj_in(".", ["git", "init", "repo"]).success();
    let work_dir = test_env.work_dir("repo");

    // Create a conflict in the working copy with long markers on one side
    work_dir.write_file(
        "file",
        indoc! {"
            line 1
            line 2
            line 3
        "},
    );
    work_dir.run_jj(["commit", "-m", "base"]).success();
    work_dir.write_file(
        "file",
        indoc! {"
            line 1
            line 2 - left
            line 3 - left
        "},
    );
    work_dir.run_jj(["commit", "-m", "side-a"]).success();
    work_dir
        .run_jj(["new", "description(base)", "-m", "side-b"])
        .success();
    work_dir.write_file(
        "file",
        indoc! {"
            line 1
            ======= fake marker
            line 2 - right
            ======= fake marker
            line 3
        "},
    );
    work_dir
        .run_jj(["new", "description(side-a)", "description(side-b)"])
        .success();

    // File should be materialized with long conflict markers
    insta::assert_snapshot!(work_dir.read_file("file"), @r"
    line 1
    <<<<<<<<<<< Conflict 1 of 1
    %%%%%%%%%%% Changes from base to side #1
    -line 2
    -line 3
    +line 2 - left
    +line 3 - left
    +++++++++++ Contents of side #2
    ======= fake marker
    line 2 - right
    ======= fake marker
    line 3
    >>>>>>>>>>> Conflict 1 of 1 ends
    ");

    // The timestamps in the `jj debug local-working-copy` output change, so we want
    // to remove them before asserting the snapshot
    let timestamp_regex = Regex::new(r"\b\d{10,}\b").unwrap();
    // On Windows, executable is always `()`, but on Unix-like systems, it's `true`
    // or `false`, so we want to remove it from the output as well
    let executable_regex = Regex::new("executable: [^ ]+").unwrap();

    let redact_output = |output: String| {
        let output = timestamp_regex.replace_all(&output, "<timestamp>");
        let output = executable_regex.replace_all(&output, "<executable>");
        output.into_owned()
    };

    // Working copy should contain conflict marker length
    let output = work_dir.run_jj(["debug", "local-working-copy"]);
    insta::assert_snapshot!(output.normalize_stdout_with(redact_output), @r#"
    Current operation: OperationId("6feb53603f9f7324085d2d89dca19a6dac93fef6795cfd5d57090ff803d404ab1196b45d5b97faa641f6a78302ac0fbd149f5e5a880d1fd64d6520c31beab213")
    Current tree: Merge(Conflicted([TreeId("381273b50cf73f8c81b3f1502ee89e9bbd6c1518"), TreeId("771f3d31c4588ea40a8864b2a981749888e596c2"), TreeId("f56b8223da0dab22b03b8323ced4946329aeb4e0")]))
    Normal { <executable> }           249 <timestamp> Some(MaterializedConflictData { conflict_marker_len: 11 }) "file"
    [EOF]
    "#);

    // Update the conflict with more fake markers, and it should still parse
    // correctly (the markers should be ignored)
    work_dir.write_file(
        "file",
        indoc! {"
            line 1
            <<<<<<<<<<< Conflict 1 of 1
            %%%%%%%%%%% Changes from base to side #1
            -line 2
            -line 3
            +line 2 - left
            +line 3 - left
            +++++++++++ Contents of side #2
            <<<<<<< fake marker
            ||||||| fake marker
            line 2 - right
            ======= fake marker
            line 3
            >>>>>>> fake marker
            >>>>>>>>>>> Conflict 1 of 1 ends
        "},
    );

    // The file should still be conflicted, and the new content should be saved
    let output = work_dir.run_jj(["st"]);
    insta::assert_snapshot!(output, @r"
    Working copy changes:
    M file
    Working copy  (@) : mzvwutvl 3a981880 (conflict) (no description set)
    Parent commit (@-): rlvkpnrz ce613b49 side-a
    Parent commit (@-): zsuskuln 7b2b03ab side-b
    Warning: There are unresolved conflicts at these paths:
    file    2-sided conflict
    [EOF]
    ");
    insta::assert_snapshot!(work_dir.run_jj(["diff", "--git"]), @r"
    diff --git a/file b/file
    --- a/file
    +++ b/file
    @@ -6,8 +6,10 @@
     +line 2 - left
     +line 3 - left
     +++++++++++ Contents of side #2
    -======= fake marker
    +<<<<<<< fake marker
    +||||||| fake marker
     line 2 - right
     ======= fake marker
     line 3
    +>>>>>>> fake marker
     >>>>>>>>>>> Conflict 1 of 1 ends
    [EOF]
    ");

    // Working copy should still contain conflict marker length
    let output = work_dir.run_jj(["debug", "local-working-copy"]);
    insta::assert_snapshot!(output.normalize_stdout_with(redact_output), @r#"
    Current operation: OperationId("205bc702428a522e0b175938a51c51b59741c854a609ba63c89de76ffda6e5eff6fcc00725328b1a91f448401769773cefcff01fac3448190d2cea4e137d2166")
    Current tree: Merge(Conflicted([TreeId("381273b50cf73f8c81b3f1502ee89e9bbd6c1518"), TreeId("771f3d31c4588ea40a8864b2a981749888e596c2"), TreeId("3329c18c95f7b7a55c278c2259e9c4ce711fae59")]))
    Normal { <executable> }           289 <timestamp> Some(MaterializedConflictData { conflict_marker_len: 11 }) "file"
    [EOF]
    "#);

    // Resolve the conflict
    work_dir.write_file(
        "file",
        indoc! {"
            line 1
            <<<<<<< fake marker
            ||||||| fake marker
            line 2 - left
            line 2 - right
            ======= fake marker
            line 3 - left
            >>>>>>> fake marker
        "},
    );

    let output = work_dir.run_jj(["st"]);
    insta::assert_snapshot!(output, @r"
    Working copy changes:
    M file
    Working copy  (@) : mzvwutvl 1aefd866 (no description set)
    Parent commit (@-): rlvkpnrz ce613b49 side-a
    Parent commit (@-): zsuskuln 7b2b03ab side-b
    [EOF]
    ");

    // When the file is resolved, the conflict marker length is removed from the
    // working copy
    let output = work_dir.run_jj(["debug", "local-working-copy"]);
    insta::assert_snapshot!(output.normalize_stdout_with(redact_output), @r#"
    Current operation: OperationId("2206ce3c108b1573df0841138c226bba1ab3cff900a5899ed31ac69162c7d6f30d37fb5ab43da60dba88047b8ab22d453887fff688f26dfcf04f2c99420a5563")
    Current tree: Merge(Resolved(TreeId("6120567b3cb2472d549753ed3e4b84183d52a650")))
    Normal { <executable> }           130 <timestamp> None "file"
    [EOF]
    "#);
}
