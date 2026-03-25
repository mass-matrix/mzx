.PHONY: clean clean-build clean-pyc clean-test dist docs format help install lint lint-fix mypy release release-test servedocs setup test uninstall
.DEFAULT_GOAL := help

define BROWSER_PYSCRIPT
import os, webbrowser, sys

from urllib.request import pathname2url

webbrowser.open("file://" + pathname2url(os.path.abspath(sys.argv[1])))
endef
export BROWSER_PYSCRIPT

define PRINT_HELP_PYSCRIPT
import re, sys

for line in sys.stdin:
	match = re.match(r'^([a-zA-Z_-]+):.*?## (.*)$$', line)
	if match:
		target, help = match.groups()
		print("%-20s %s" % (target, help))
endef
export PRINT_HELP_PYSCRIPT

BROWSER := python -c "$$BROWSER_PYSCRIPT"

## remove all build, test, coverage and Python artifacts
clean: clean-build clean-pyc clean-test

clean-build: ## remove build artifacts
	rm -fr build/
	rm -fr dist/
	rm -fr .eggs/
	find . -name '*.egg-info' -exec rm -fr {} +
	# find . -name '*.egg' -exec rm -f {} +

clean-pyc: ## remove Python file artifacts
	find . -name '*.pyc' -exec rm -f {} +
	find . -name '*.pyo' -exec rm -f {} +
	find . -name '*~' -exec rm -f {} +
	find . -name '__pycache__' -exec rm -fr {} +

clean-test: ## remove test and coverage artifacts
	rm -fr .tox/
	rm -f .coverage
	rm -fr htmlcov/
	rm -fr .pytest_cache

dist: clean ## builds source and wheel package
	python setup.py sdist
	python setup.py bdist_wheel
	ls -l dist

docs: ## generate Sphinx HTML documentation, including API docs
	rm -f docs/mzx.rst
	rm -f docs/modules.rst
	sphinx-apidoc -o docs/ src/mzx
	$(MAKE) -C docs clean
	$(MAKE) -C docs html
	$(BROWSER) docs/_build/html/index.html

format: ## format source with ruff
	ruff format .

help: ## show this help
	@python -c "$$PRINT_HELP_PYSCRIPT" < $(MAKEFILE_LIST)

install: ## editable install of mzx (use after `make setup` or with uv/pip in your env)
	uv pip install -e .

lint: ## run ruff linter
	ruff check .

lint-fix: ## auto-fix ruff issues where possible
	ruff check --fix .

mypy: ## typecheck src/
	mypy --strict-optional --check-untyped-defs --disallow-incomplete-defs --warn-unused-configs --follow-imports=normal src/

release: dist ## package and upload a release
	twine upload --verbose dist/*

release-test: dist ## package and upload a release
	twine upload --verbose --repository pypitest dist/*

servedocs: docs ## compile the docs watching for changes
	watchmedo shell-command -p '*.rst' -c '$(MAKE) -C docs html' -R -D .

setup: ## create .venv and install dev dependencies from requirements.txt
	uv venv
	uv pip install -r requirements.txt

test: ## run tests with coverage
	python -m pytest --capture=sys --capture=fd --cov=src/ -vv tests/

uninstall: ## remove mzx from the active environment
	uv pip uninstall mzx
