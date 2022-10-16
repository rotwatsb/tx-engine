File.open('test2.csv', 'w') do |file|
  file.write("type, client, tx, amount\n")

  for i in 1..10000000 do
    file.write("deposit, 1, #{i}, 1\n")
  end
end
