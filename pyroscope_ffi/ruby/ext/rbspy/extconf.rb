require 'mkmf'
require 'rb_sys/mkmf'
require 'rake'

create_rust_makefile('rbspy')

app = Rake.application
app.init
app.add_import 'Rakefile'
app.load_rakefile

app['rbspy'].invoke
