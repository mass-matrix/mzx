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

To also install the Rust backend, which converts supported Thermo files without
Docker (see `Rust backend`_):

.. code-block:: console

        pip install -U "mzx[rust]"

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

Rust backend
------------

``rust/`` contains a pure-Rust Thermo RAW → mzML converter, ``mzx-rs``, that runs
without Docker. It writes output only when that output is **proven byte-identical**
to the pinned msconvert, after normalizing the converter's name, version and command
line and the input file location (see ``rust/CONTRACT.md``). Proof means
comparison against msconvert goldens for the files and option sets listed in
``rust/support_matrix.toml``. Today that is:

* Q Exactive Focus files with the default options and with no filters.
* LTQ XL ion-trap files with the default options, with no filters, and with peak
  picking on all MS levels (with or without zero removal). Peak picking of their
  profile scans uses mzx's model of Thermo's ion-trap centroider, which matches
  every centroid of the corpus bit for bit.

For anything else the Rust backend refuses, and mzx falls back to msconvert in Docker.

``mzx-rs`` is a separate package, installed with the ``rust`` extra:

.. code-block:: console

        pip install "mzx[rust]"

There are no prebuilt wheels yet, so pip compiles ``mzx-rs`` from source. That
needs a C compiler: gcc or clang on Linux, the Xcode command line tools on macOS
(``xcode-select --install``), or the Visual Studio Build Tools ("Desktop
development with C++") on Windows. A missing Rust toolchain and CMake are
downloaded automatically during the build; the first install takes a few
minutes. If the build fails, plain ``pip install mzx`` still works with Docker.

From a clone, build and install it into the same environment as mzx instead:

.. code-block:: console

        pip install ./rust/crates/mzx-py

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
