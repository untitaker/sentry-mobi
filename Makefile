build: static/htmx.js static/pico.css static/pico.colors.css static/htmx.preload.js
	cargo build

.PHONY: build

static/htmx.js:
	curl -Lf https://unpkg.com/htmx.org@2.0.3 -o $@

static/pico.css:
	curl -Lf https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.violet.min.css -o $@

static/pico.colors.css:
	curl -Lf https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.colors.min.css -o $@

static/htmx.preload.js:
	# https://github.com/bigskysoftware/htmx-extensions/issues/108
	curl -Lf https://raw.githubusercontent.com/marisst/htmx-extensions/refs/heads/main/src/preload/preload.js -o $@
