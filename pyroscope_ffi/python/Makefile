.PHONY: clean
clean:
	rm -rf build
	rm -rf dist
	rm -rf wheelhouse
	rm -rf pyroscope_beta.egg-info
	rm -rf pyroscope_io.egg-info
	rm -rf .eggs
	rm -rf lib/target

.PHONY: build
build: clean
	python -m build --wheel


.PHONY: bdist_wheel
bdist_wheel: clean
	python setup.py bdist_wheel

.PHONY: install
install: build
	pip install --force-reinstall dist/*.whl
