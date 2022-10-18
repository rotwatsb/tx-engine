File.open('test3.csv', 'w') do |file|
  file.write("type, client, tx, amount\n")
  file.write("deposit, 1, 1, 10000000\n")

  for i in 1..10000000 do
    file.write("withdrawal, 1, #{i + 1}, 1\n")
  end
end
