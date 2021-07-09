desc 'Format code with nightly cargo fmt'
task :format do
  sh 'cargo +nightly fmt'
end

desc 'Check'
task :check do
  sh 'cargo +nightly fmt -- --color=always --check'
  sh 'cargo clippy'
  sh 'cargo test'
end
