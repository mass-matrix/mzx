.. highlight:: shell

============
Installation
============


Stable release
--------------

To install the latest release:

.. code-block:: console

    $ pip install mzx

To also install the Rust backend (``mzx-rs``), which converts supported Thermo
files without Docker:

.. code-block:: console

    $ pip install "mzx[rust]"

``mzx-rs`` is compiled from source during the install, which needs a C compiler
(gcc or clang on Linux, the Xcode command line tools on macOS, the Visual Studio
Build Tools on Windows). pip downloads a Rust toolchain and CMake when they are
missing. Without the extra, or if its build fails, mzx uses Docker for every
conversion. See "Rust backend" in the README for what it covers.

Use a virtual environment if you do not want packages installed into your system Python. If you need help installing Python or ``pip``, see the `Python packaging user guide <https://packaging.python.org/en/latest/tutorials/installing-packages/>`_.


Prerequisites
-------------

* **Python 3.10 or newer**
* **Docker** — must be installed and the daemon running. mzx runs ProteoWizard's ``msconvert`` inside a container (see the main README for the image name). With the ``rust`` extra, conversions the Rust backend supports do not need Docker.


From source (contributors)
--------------------------

Clone the repository (HTTPS works without SSH keys):

.. code-block:: console

    $ git clone https://github.com/mass-matrix/mzx.git
    $ cd mzx

Create a virtual environment and install dependencies, then install mzx in editable mode:

.. code-block:: console

    $ python -m venv .venv
    $ source .venv/bin/activate
    $ pip install -r requirements.txt
    $ pip install -e .
    $ pip install ./rust/crates/mzx-py   # optional: the Rust backend

If you use `uv <https://docs.astral.sh/uv/>`_, you can run ``make setup`` and ``make install`` from the project root instead (see ``README.rst``). ``uv pip install -e ".[rust]"`` (or ``make install-rust``) also builds the Rust backend from ``rust/crates/mzx-py`` in the clone.
