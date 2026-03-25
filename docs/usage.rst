=====
Usage
=====

Command line
------------

Default: convert a raw file to mzML in the same directory as the input:

.. code-block:: console

  mzx /path/to/data.raw

Other output formats:

.. code-block:: console

  mzx --type mgf /path/to/data.raw
  mzx --type mzxml /path/to/data.raw

Full options:

.. code-block:: console

  mzx --help

The same CLI is available as:

.. code-block:: console

  python -m mzx

GUI
---

.. code-block:: console

  mzx-gui

If ``mzx-gui`` is not on your ``PATH`` (e.g. some Windows setups), use:

.. code-block:: console

  python -m mzx.gui

The GUI is experimental; the CLI is recommended for scripting and automation.

License
-------

mzx is licensed under the GNU General Public License v3.0. See the ``LICENSE`` file in the repository.

* Documentation: https://mzx.readthedocs.io
