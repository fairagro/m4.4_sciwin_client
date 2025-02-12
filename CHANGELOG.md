# Unreleased
## New Features
- Allowed handling of nullable and array CWLTypes using `File?` or `File[]` notation
- Added `s4n execute make-template ./path/to.cwl` to create job templates #75

## Bugfixes
- Fixed setting correct InitialWorkDirRequirement when `-i` is used in `s4n tool create` #69
- Fixed handling of json-Data #60
- Fixed unreported Bug, where CWL CommandLineTool Output was ignored if not of type File, Directory, stdout, stderr or string. 781d20e

## Other
- Added Tests for all Documentation examples #76
- Added CWL Conformance Tests to CI Workflow

# 0.1.0 (2025-02-04)
Initial Release
