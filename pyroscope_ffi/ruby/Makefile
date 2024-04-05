
.phony: dev
dev:
	cd ext/rbspy &&  cargo build
	cd ext/thread_id && cargo build
	mkdir -p lib/rbspy/
	cp ./ext/rbspy/target/debug/librbspy.so lib/rbspy/rbspy.so
	mkdir -p lib/thread_id/
	cp ./ext/thread_id/target/debug/libthread_id.so lib/thread_id/thread_id.so
	rake x86_64_linux:gem # todo wth is it so slow
	for i in $$(ls -tr pkg/pyroscope-*-x86_64-linux.gem | tail -n 1); do gem install ./$$i; done;

.phony: clean
clean:
	rm -rf pkg lib/thread_id/*.so lib/rbspy/*.so
	cd ext/rbspy && cargo clean
	cd ext/thread_id && cargo clean
