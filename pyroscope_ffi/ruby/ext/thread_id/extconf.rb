require 'mkmf'
require 'rb_sys/mkmf'
require 'rake'

create_rust_makefile('thread_id')

#app = Rake.application
#app.init
#app.add_import 'Rakefile'
#app.load_rakefile

#app['thread_id'].invoke
