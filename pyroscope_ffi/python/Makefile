.PHONY: clean
clean:
	rm -rf build
	rm -rf dist
	rm -rf wheelhouse
	rm -rf pyroscope_beta.egg-info

.PHONY: build
build: clean
	python -m build --wheel

.PHONY: install
install: build
	pip install --force-reinstall dist/*.whl
