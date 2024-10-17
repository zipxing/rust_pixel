run: build 
	python3 -m http.server -d . 8080

build:
	wasm-pack build --target web --release
