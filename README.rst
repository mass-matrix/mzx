===============================
mzx
===============================
        Free and open conversion of mass spec data

.. image:: https://img.shields.io/pypi/v/mzx.svg
        :target: https://pypi.python.org/pypi/mzx
        :alt: PyPI

.. image:: https://github.com/mass-matrix/mzx/actions/workflows/pytest.yml/badge.svg
        :target: https://github.com/mass-matrix/mzx/actions/workflows/pytest.yml
        :alt: GitHub Actions

.. image:: https://readthedocs.org/projects/mzx/badge/?version=latest
    :target: https://mzx.readthedocs.io/en/latest/?badge=latest
    :alt: Documentation Status

.. image:: https://codecov.io/gh/mass-matrix/mzx/graph/badge.svg?token=mrLdM9zX54
        :target: https://codecov.io/gh/mass-matrix/mzx
        :alt: Codecov

.. image:: https://img.shields.io/pypi/dm/mzx
        :alt: PyPI - Downloads


What it does
------------

**mzx** wraps `msconvert` from `ProteoWizard <https://proteowizard.sourceforge.io/>`_ inside Docker so you can convert vendor raw formats to open formats (mzML, MGF, mzXML, …) from the command line or an optional GUI.

Prerequisites
-------------

* **Docker** — installed *and running* (the CLI calls ``docker run`` to execute msconvert).
* **Python 3.10+**
* **pip**, **uv**, or another PEP 517–compatible installer

Install
-------

.. code-block:: console

        pip install -U mzx

Quick start
-----------

#. Ensure Docker is running (``docker info`` should succeed).
#. Convert a Thermo ``.raw`` file to mzML (default output format):

   .. code-block:: console

        mzx /path/to/data.raw

   The mzML is written next to the input file (same directory, ``.mzML`` extension).

#. To choose another output format, set ``--type`` to ``mzml``, ``mgf``, or ``mzxml``:

   .. code-block:: console

        mzx --type mgf /path/to/data.raw

See ``mzx --help`` for peak picking, indexing, Waters lockmass options, and more.

GUI (experimental)
------------------

After install, start the GUI:

.. code-block:: console

        mzx-gui

If the ``mzx-gui`` command is not on your ``PATH`` (common on Windows), use either:

.. code-block:: console

        python -m mzx.gui

or the Python Launcher for Windows:

.. code-block:: doscon

        py -m mzx.gui

The CLI can always be run as ``python -m mzx`` (same as the ``mzx`` command).

Vendor support
--------------

Conversion uses ProteoWizard; supported vendors include Agilent, Bruker, Sciex, Shimadzu, Thermo, Waters, and UIMF. See the `ProteoWizard FAQ <https://proteowizard.sourceforge.io/faq.html>`_ for vendor-specific notes.

Rust backend (experimental)
---------------------------

``rust/`` contains a pure-Rust Thermo RAW → mzML converter, ``mzx-rs``, that runs
without Docker. It writes output only when that output is **proven byte-identical**
to the pinned msconvert, after normalizing the converter's name, version and command
line and the input file location (see ``rust/CONTRACT.md``). Proof means
comparison against msconvert goldens for the files and option sets listed in
``rust/support_matrix.toml``. Today that is Q Exactive Focus files (with
the default options and with no filters) and LTQ XL files with no filters.
For anything else the Rust backend refuses, and mzx falls back to msconvert in Docker.

Build and install it into the same environment as mzx (needs a Rust toolchain):

.. code-block:: console

        pip install maturin
        make rust-wheel
        pip install rust/dist/mzx_rs-*.whl

Choose the backend with ``--backend``:

* ``auto`` (default): Rust when ``mzx-rs`` is installed and supports the file and
  options, otherwise msconvert in Docker.
* ``rust``: Rust only; fails instead of falling back.
* ``docker``: always msconvert.

The Rust backend ignores the msconvert ``timeout`` setting.

Supported file formats (examples)
---------------------------------

* ``.mgf``
* ``.mzML``
* ``.raw`` (Thermo)
* ``.wiff`` (Sciex)

Features
--------

* Convert between common mass spectrometry interchange formats via msconvert
* Vendor formats: Agilent, Bruker, Sciex, Thermo, Waters, and others supported by ProteoWizard
* CLI and experimental GUI

Documentation
-------------

Full documentation: `mzx.readthedocs.io <https://mzx.readthedocs.io/en/latest>`_

Development
-----------

From a clone of the repo, using `uv <https://docs.astral.sh/uv/>`_ (recommended):

.. code-block:: console

        make setup
        source .venv/bin/activate   # Windows: .venv\Scripts\activate
        make install

``make setup`` creates ``.venv`` and installs development dependencies from ``requirements.txt``. ``make install`` installs **mzx** in editable mode.

Without ``make``, the same steps are:

.. code-block:: console

        uv venv
        source .venv/bin/activate
        uv pip install -r requirements.txt
        uv pip install -e .

Run tests:

.. code-block:: console

        make test

Other useful targets: ``make lint``, ``make help``.

License
-------

GNU General Public License v3 — see ``LICENSE``.
