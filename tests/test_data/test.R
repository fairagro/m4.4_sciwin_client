load("example_input.RData")

data <- data.frame(
  ID = 1:5,
  Name = c("F", "G", "H", "I", "J"),
  Score = c(5, 10, 12, 18, 23)
)

# Save it as an RData file
save(data, file = "example.RData")

