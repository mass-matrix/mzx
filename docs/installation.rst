.. highlight:: shell

============
Installation
============


Stable release
--------------

To install the latest release:

.. code-block:: console

    $ pip install mzx

Use a virtual environment if you do not want packages installed into your system Python. If you need help installing Python or ``pip``, see the `Python packaging user guide <https://packaging.python.org/en/latest/tutorials/installing-packages/>`_.


Prerequisites
-------------

* **Python 3.10 or newer**
* **Docker** — must be installed and the daemon running. mzx runs ProteoWizard's ``msconvert`` inside a container (see the main README for the image name).


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

If you use `uv <https://docs.astral.sh/uv/>`_, you can run ``make setup`` and ``make install`` from the project root instead (see ``README.rst``).
