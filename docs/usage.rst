=====
Usage
=====

Command line
------------

Default conversion (Docker / ProteoWizard)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Convert a raw file to mzML in the same directory as the input:

.. code-block:: console

  mzx /path/to/data.raw

Other output formats (Docker path only):

.. code-block:: console

  mzx --type mgf /path/to/data.raw
  mzx --type mzxml /path/to/data.raw

Full options:

.. code-block:: console

  mzx --help

The same CLI is available as:

.. code-block:: console

  python -m mzx

Native conversion (experimental)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Convert vendor files to mzML without Docker using optional native parsers.
Install extras first (see :doc:`installation`), then pass ``--native``:

.. code-block:: console

  pip install mzx[native]
  mzx --native /path/to/data.raw
  mzx --native --vendor waters /path/to/sample.raw/
  mzx --native --output /tmp/out.mzML /path/to/data.raw

Supported native vendors: Thermo, Waters, Agilent, and Bruker. Native conversion
writes **mzML only** (``--type mgf`` and ``--type mzxml`` are rejected with
``--native``). Output may differ from ProteoWizard/msconvert.

Python API
~~~~~~~~~~

Use :func:`mzx.convert_file` to choose the conversion path programmatically:

.. code-block:: python

  from mzx import convert_file

  params = {
      "infile": "/path/to/data.raw",
      "vendor": "Thermo",
      "type": "mzml",
      "outfile": None,
      "index": False,
      "sortbyscan": False,
      "peak_picking": "msms",
      "remove_zeros": True,
      "overwrite": False,
      "debug": False,
      "verbose": False,
      "lockmass_disabled": True,
      "lockmass": False,
      "neg_lockmass": None,
      "pos_lockmass": None,
      "lockmass_tolerance": None,
      "lockmass_function_exclude": None,
  }

  # Default: ProteoWizard via Docker
  out = convert_file(params)

  # Experimental: native parsers, no Docker
  out = convert_file(params, native=True)

Lower-level native API:

.. code-block:: python

  from mzx.convert import convert_native, detect_vendor, list_converters

  vendor = detect_vendor("/path/to/data.raw")
  converters = list_converters()
  out = convert_native(params)

Native limitations
~~~~~~~~~~~~~~~~~~

+---------------------------+-----------------------------------------------+
| Topic                     | Notes                                         |
+===========================+===============================================+
| Output formats            | mzML only                                     |
| Vendors                   | Thermo, Waters, Agilent, Bruker               |
| Sciex / Shimadzu          | Not supported natively                        |
| Peak picking / lockmass   | Lightweight filters; not identical to PWiz    |
| Optional dependencies     | ``pip install mzx[native]`` or per-vendor     |
+---------------------------+-----------------------------------------------+

GUI
---

.. code-block:: console

  mzx-gui

If ``mzx-gui`` is not on your ``PATH`` (e.g. some Windows setups), use:

.. code-block:: console

  python -m mzx.gui

The GUI is experimental and uses the Docker/msconvert path only; the CLI is
recommended for scripting and automation.

License
-------

mzx is licensed under the GNU General Public License v3.0. See the ``LICENSE`` file in the repository.

* Documentation: https://mzx.readthedocs.io
