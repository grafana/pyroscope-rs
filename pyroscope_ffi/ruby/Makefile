
.phony: dev
dev:
	cd ext/rbspy &&  cargo build --frozen --offline
	mkdir -p lib/rbspy/
	cp ../../target/debug/librbspy.so lib/rbspy/rbspy.so
	bundle install
	rake x86_64_linux:gem # todo wth is it so slow
	for i in $$(ls -tr pkg/pyroscope-*-x86_64-linux.gem | tail -n 1); do gem install ./$$i; done;

.phony: clean
clean:
	rm -rf pkg lib/thread_id/*.so lib/rbspy/*.so
