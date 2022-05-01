require 'mkmf'
require 'rake'

create_makefile('thread_id')

app = Rake.application
app.init
app.add_import 'Rakefile'
app.load_rakefile

app['default'].invoke
