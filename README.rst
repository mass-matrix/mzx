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


Installing
----------

System Requirements:

* Docker
* Python3.10+ (Contains `PEP 636 <https://peps.python.org/pep-0636/>`_ )
* Your favorite Python package manager (uv, pip, poetry, ...)

Install and update using `pip`\:

.. code-block:: console

        pip install -U mzx

Usage
-----

To run the cli command:

.. code-block:: console

        mzx --type mgf /path/to/data.mgf

This will convert the `mgf` data to `mzml` by default.

To run the gui:

.. code-block:: console

        mzx-gui

Note: The gui is experimental

You can also call the command from the module itself:

.. code-block:: console

        python -m mzx.cli /path/to/data.raw

.. code-block:: console

        python -m mzx.gui

Vendor Support
--------------

mzx utilizes proteowizard, and supports the following vendors: Agilent, Bruker, Sciex, Shimadzu, Thermo, and UIMF.

For more information, please see the `proteowizard FAQ <https://proteowizard.sourceforge.io/faq.html>`

Supported File Formats
----------------------
* .mgf
* .mzML
* .raw (Thermo)
* .wiff (Sciex)

Features
--------
* Convert between various mass spectrometry file formats
* Supports vendor formats: Agilent, Bruker, Sciex, etc.
* CLI and experimental GUI for ease of use

Documentation
-------------
Full documentation is available at `mzx.readthedocs.io <https://mzx.readthedocs.io/en/latest>`_

Developer setup
---------------

If you have uv installed, then you can a venv setup by running\:

.. code-block:: console

        make setup

While making changes to mzx, you can install/uninstall it from your local

.. code-block:: console

        make install

.. code-block:: console

        make uninstall

Tests
-----

.. code-block:: console

        make test
