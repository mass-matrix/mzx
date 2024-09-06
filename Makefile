.PHONY: install mypy setup test uninstall

install:
	uv pip install -e .

mypy:
	mypy --strict-optional --check-untyped-defs --disallow-incomplete-defs --warn-unused-configs --follow-imports=normal src/

setup:
	uv venv
	uv pip install -r requirements.txt

test:
	python -m pytest --capture=sys --capture=fd --cov=src/ -vv tests/

uninstall:
	uv pip uninstall mmconvert
