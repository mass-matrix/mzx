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

**mzx** converts vendor mass spectrometry files to open formats from the command line
or an optional GUI. Two paths are available:

* **Default** — wraps `msconvert` from `ProteoWizard <https://proteowizard.sourceforge.io/>`_
  inside Docker (mzML, MGF, mzXML, …).
* **Experimental native** — pure-Python/Rust vendor parsers with no Docker
  (``--native``; mzML output only).

Prerequisites
-------------

* **Python 3.10+**
* **pip**, **uv**, or another PEP 517–compatible installer
* **Docker** — required for the default path (``docker info`` must succeed).
  Not required when using ``--native``.

Install
-------

.. code-block:: console

        pip install -U mzx

For experimental native conversion, install optional vendor parsers:

.. code-block:: console

        pip install mzx[native]     # Thermo, Waters, Agilent, Bruker
        pip install mzx[thermo]     # or per-vendor extras

Quick start
-----------

Default path (Docker / ProteoWizard)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

#. Ensure Docker is running (``docker info`` should succeed).
#. Convert a Thermo ``.raw`` file to mzML (default output format):

   .. code-block:: console

        mzx /path/to/data.raw

   The mzML is written next to the input file (same directory, ``.mzML`` extension).

#. To choose another output format, set ``--type`` to ``mzml``, ``mgf``, or ``mzxml``:

   .. code-block:: console

        mzx --type mgf /path/to/data.raw

See ``mzx --help`` for peak picking, indexing, Waters lockmass options, and more.

Native conversion (experimental)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Convert without Docker using built-in vendor parsers (mzML only):

.. code-block:: console

        pip install mzx[native]
        mzx --native /path/to/data.raw
        mzx --native --vendor waters /path/to/sample.raw/
        mzx --native --output /tmp/out.mzML /path/to/data.raw

Supported native vendors: Thermo, Waters, Agilent, and Bruker. Output may differ
from ProteoWizard/msconvert. See ``docs/usage.rst`` for the Python API.

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

ProteoWizard (default)
~~~~~~~~~~~~~~~~~~~~~~

The default Docker path supports Agilent, Bruker, Sciex, Shimadzu, Thermo, Waters,
and UIMF. See the `ProteoWizard FAQ <https://proteowizard.sourceforge.io/faq.html>`_
for vendor-specific notes.

Native (experimental)
~~~~~~~~~~~~~~~~~~~~~

The ``--native`` path supports Thermo ``.raw`` files, Waters ``.raw/`` directories,
Agilent ``.d/`` directories, and Bruker timsTOF ``.d/`` bundles. Sciex and Shimadzu
are not supported natively.

Supported file formats (examples)
---------------------------------

* ``.mgf``
* ``.mzML``
* ``.raw`` (Thermo)
* ``.wiff`` (Sciex)

Features
--------

* Convert between common mass spectrometry interchange formats via msconvert (default)
* Experimental native conversion without Docker (``--native``; mzML only)
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
