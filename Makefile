clean-build: ## remove build artifacts
	rm -fr build/
	rm -fr dist/
	rm -fr .eggs/
	find . -name '*.egg-info' -exec rm -fr {} +
	find . -name '*.egg' -exec rm -f {} +

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

help:
	@python -c "$$PRINT_HELP_PYSCRIPT" < $(MAKEFILE_LIST)

install:
	uv pip install -e .

lint:
	ruff check .

mypy:
	mypy --strict-optional --check-untyped-defs --disallow-incomplete-defs --warn-unused-configs --follow-imports=normal src/

release: dist ## package and upload a release
	twine upload dist/*

setup:
	uv venv
	uv pip install -r requirements.txt

test:
	python -m pytest --capture=sys --capture=fd --cov=src/ -vv tests/

uninstall:
	uv pip uninstall mmconvert
