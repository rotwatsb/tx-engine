File.open('test6.csv', 'w') do |file|
  file.write("type, client, tx, amount\n")

  for i in 1..10 do
    file.write("deposit, #{i}, #{2*i - 1}, 10\n")
    file.write("withdrawal, #{i}, #{2*i},  #{i}\n")
  end

  for i in 1..2 do
    file.write("dispute, #{i}, #{2*i - 1}\n")
    file.write("dispute, #{i}, #{2*i - 1}\n")
  end

  for i in 1..1 do
    file.write("resolve, #{i}, #{2*i - 1}\n")
  end

  for i in 2..2 do
    file.write("chargeback, #{i}, #{2*i - 1}\n")
  end

  # make sure that locked accounts cannot be deposited into
  for i in 2..2 do
    file.write("deposit, #{i}, #{2*i - 1}, 10\n")
  end
end
