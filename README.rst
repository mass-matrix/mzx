===============================
mzx
===============================
        Free and open conversion of mass spec data

.. _msconvert: https://proteowizard.sourceforge.io/

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

mzx is designed to work with raw, tdf

Installing
----------

Install and update using `pip`\:

.. code-block:: console

        pip install -U mzx

Usage
-----

.. code-block:: console

        mzx --type mgf /path/to/data.mgf

This will convert the `mgf` data to `mzml` by default.

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
