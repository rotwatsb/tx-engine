File.open('test4.csv', 'w') do |file|
  file.write("type, client, tx, amount\n")

  for i in 1..10 do
    file.write("deposit, #{i}, #{2*i - 1}, 10\n")
    file.write("withdrawal, #{i}, #{2*i},  #{i}\n")
  end
end
