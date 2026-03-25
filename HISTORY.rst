=======
History
=======

0.3.2 (2026-03-25)
------------------
* Documentation: README and Sphinx docs updated for clearer setup, prerequisites (Docker, Python 3.10+), installation from source, and usage; fixed incorrect license line in usage docs (GPLv3); corrected CONTRIBUTING workflow (ruff, pytest, ``make`` targets) and removed stale template text.
* Packaging: add ``python -m mzx`` as the CLI entry (``__main__.py``); register ``mzx-gui`` under ``gui_scripts`` so Windows installs a GUI launcher via ``pythonw`` (no extra console window); document ``python -m mzx.gui`` when debugging or when scripts are not on ``PATH``.
* Project metadata: PyPI “Documentation” URL now points to https://mzx.readthedocs.io/en/latest/ .
* CI: install the package in editable mode before running tests so ``import mzx`` works; remove obsolete ``requirements.txt`` ``sed`` workaround from the workflow.
* Makefile: ``help`` text for common targets (``install``, ``setup``, ``test``, etc.).
* Tests: expanded coverage for ``msconvert``, ``convert_raw_file``, ``waters_convert``, CLI, ``run_cmd``, ``exclusion_string`` / Waters helpers, and ``python -m mzx``; tests import the ``mzx`` package instead of ``src.mzx``.

**Full Changelog**: https://github.com/mass-matrix/mzx/compare/0.3.1...0.3.2

0.3.1 (2025-06-26)
------------------
* Bump setuptools from 75.1.0 to 78.1.1 in the pip group across 1 directory by @dependabot in https://github.com/mass-matrix/mzx/pull/27
* Fixes mypy errors. by @neonelemental in https://github.com/mass-matrix/mzx/pull/32
* cleaned up waters parsing and added additional waters specific options by @mafreitas in https://github.com/mass-matrix/mzx/pull/28

## New Contributors
* @neonelemental made their first contribution in https://github.com/mass-matrix/mzx/pull/32

**Full Changelog**: https://github.com/mass-matrix/mzx/compare/0.3.0...0.3.1

0.3.0 (2025-05-19)
------------------
* Update GUI to allow directory + icon by @Kartstig in https://github.com/mass-matrix/mzx/pull/21
* added params to gui by @mafreitas in https://github.com/mass-matrix/mzx/pull/25
* Bump cryptography from 43.0.1 to 44.0.1 in the pip group across 1 directory by @dependabot in https://github.com/mass-matrix/mzx/pull/22

## New Contributors
* @dependabot made their first contribution in https://github.com/mass-matrix/mzx/pull/22

**Full Changelog**: https://github.com/mass-matrix/mzx/compare/0.2.3...0.3.0

0.2.3 (2024-05-07)
------------------
* Handle trailing slashes when passed directory by @Kartstig in https://github.com/mass-matrix/mzx/pull/20

**Full Changelog**: https://github.com/mass-matrix/mzx/compare/0.2.2...0.2.3

0.2.2 (2024-11-15)
------------------

* Mzx fix by @mafreitas in https://github.com/mass-matrix/mzx/pull/16
* Fixes a bug where Waters file encoding causes an error
* @mafreitas made their first contribution in https://github.com/mass-matrix/mzx/pull/16

**Full Changelog**: https://github.com/mass-matrix/mzx/compare/0.2.1...0.2.2

0.2.1 (2024-10-23)
------------------

* Package fixes by @Kartstig in https://github.com/mass-matrix/mzx/pull/14


**Full Changelog**: https://github.com/mass-matrix/mzx/compare/0.2.0...0.2.1

0.2.0 (2024-10-16)
------------------

* gui: Added visual for drag/drop
* gui: Run conversion in background thread
* Update README by @Kartstig in https://github.com/mass-matrix/mzx/pull/12
* UI fixes by @Kartstig in https://github.com/mass-matrix/mzx/pull/13


**Full Changelog**: https://github.com/mass-matrix/mzx/compare/0.1.0...0.2.0

0.1.0 (2024-10-14)
------------------

* First release on PyPI.
