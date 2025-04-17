# Unreleased

# v0.3.0
## 🚀 Features
- Added Containerization Support (Docker & Podman) for `s4n execute local`
- Support CWL ExpressionTools

## 🐛 Bugfixes
- Fixed some bugs in CWL Runner ramping up its conformance from 90/378 to 126/378

## 👀 Miscellaneous Tasks
- Removed Nightly Builds CI Workflow

# v0.2.0
## 🚀 Features
- Allowed handling of nullable and array CWLTypes using `File?` or `File[]` notation
- Added `s4n execute make-template ./path/to.cwl` to create job templates #75
- Added support for the direct execution of files #79
- Allow Directories as output

## 🐛 Bugfixes
- Fixed setting correct InitialWorkDirRequirement when `-i` is used in `s4n tool create` #69
- Fixed handling of json-Data #60
- Fixed unreported Bug, where CWL CommandLineTool Output was ignored if not of type File, Directory, stdout, stderr or string. 781d20e
- Fixed Command fail because of invalid git user config - prompts user if missing #78
- Fixed cleanup if init fails #77
- Fixed Files in subfolders can not be created in s4n tool create #88
- Fixed Do not check for uncommited changes if --no-run #89

## 🚜 Refactor
- Moved Runner into separate crate (Refactor)

## 👀 Miscellaneous Tasks
- Added Tests for all Documentation examples #76
- Added CWL Conformance Tests to CI Workflow
- Added more integration tests

# v0.1.0
Initial Release
