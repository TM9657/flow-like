resource "null_resource" "build_and_push_lambda" {
  provisioner "local-exec" {
    command = <<EOT
      docker build -t ${aws_ecr_repository.lambda_repo.repository_url}:latest ./docker/lambda
      aws ecr get-login-password --region ${var.region} | docker login --username AWS --password-stdin ${aws_ecr_repository.lambda_repo.repository_url}
      docker push ${aws_ecr_repository.lambda_repo.repository_url}:latest
    EOT
  }
  depends_on = [aws_ecr_repository.lambda_repo]
}

resource "null_resource" "build_and_push_api" {
  provisioner "local-exec" {
    command = <<EOT
      docker build -t ${aws_ecr_repository.api_repo.repository_url}:latest ./docker/api
      aws ecr get-login-password --region ${var.region} | docker login --username AWS --password-stdin ${aws_ecr_repository.api_repo.repository_url}
      docker push ${aws_ecr_repository.api_repo.repository_url}:latest
    EOT
  }
  depends_on = [aws_ecr_repository.api_repo]
}

resource "null_resource" "build_and_push_runtime" {
  provisioner "local-exec" {
    command = <<EOT
      docker build -t ${aws_ecr_repository.runtime_repo.repository_url}:latest ./docker/runtime
      aws ecr get-login-password --region ${var.region} | docker login --username AWS --password-stdin ${aws_ecr_repository.runtime_repo.repository_url}
      docker push ${aws_ecr_repository.runtime_repo.repository_url}:latest
    EOT
  }
  depends_on = [aws_ecr_repository.runtime_repo]
}

resource "aws_s3_bucket" "cdn_bucket" {
  bucket = "cdn-bucket-${random_string.suffix.result}"
  acl    = "private"

  server_side_encryption_configuration {
    rule {
      apply_server_side_encryption_by_default {
        sse_algorithm = "AES256"
      }
    }
  }

  tags = {
    Name = "CDN Bucket"
  }
}

resource "aws_s3_bucket" "runtime_files_bucket" {
  bucket = "runtime-files-bucket-${random_string.suffix.result}"
  acl    = "private"

  server_side_encryption_configuration {
    rule {
      apply_server_side_encryption_by_default {
        sse_algorithm = "AES256"
      }
    }
  }

  policy = aws_s3_bucket.cdn_bucket.policy

  tags = {
    Name = "Runtime Bucket"
  }
}

resource "random_string" "suffix" {
  length  = 8
  special = false
}

resource "aws_ecr_repository" "lambda_repo" {
  name = "lambda-repo"
}

resource "aws_ecr_repository" "api_repo" {
  name = "api-repo"
}

resource "aws_ecr_repository" "runtime_repo" {
  name = "runtime-repo"
}

resource "aws_ecs_cluster" "fargate_cluster" {
  name = "fargate-cluster"
}

# Task Definitions
resource "aws_ecs_task_definition" "api_task" {
  family                   = "api-task"
  container_definitions    = jsonencode([{
    name      = "api-container",
    image     = aws_ecr_repository.api_repo.repository_url,
    memory    = 512,
    cpu       = 256,
    essential = true,
    portMappings = [{
      containerPort = 80,
      hostPort      = 80
    }]
  }])
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  memory                   = "512"
  cpu                      = "256"
  execution_role_arn       = aws_iam_role.ecs_task_execution_role.arn
  task_role_arn            = aws_iam_role.ecs_task_role.arn
}

resource "aws_ecs_task_definition" "runtime_task" {
  family                   = "runtime-task"
  container_definitions    = jsonencode([{
    name      = "runtime-container",
    image     = aws_ecr_repository.runtime_repo.repository_url,
    memory    = 4096,
    cpu       = 1024,
    essential = true
  }])
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  memory                   = "4096"
  cpu                      = "1024"
  execution_role_arn       = aws_iam_role.ecs_task_execution_role.arn
  task_role_arn            = aws_iam_role.ecs_task_role.arn
}

resource "aws_lambda_function" "lambda_function" {
  function_name = "lambda-function"
  role          = aws_iam_role.lambda_execution_role.arn
  handler       = "app.lambda_handler"
  runtime       = "provided.al2"
  package_type  = "Image"
  image_uri     = aws_ecr_repository.lambda_repo.repository_url
}

resource "aws_iam_role" "ecs_task_execution_role" {
  name = "ecs-task-execution-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17",
    Statement = [
      {
        Effect = "Allow",
        Principal = { Service = "ecs-tasks.amazonaws.com" },
        Action = "sts:AssumeRole"
      }
    ]
  })

  managed_policy_arns = [
    "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
  ]
}

resource "aws_iam_role" "ecs_task_role" {
  name = "ecs-task-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17",
    Statement = [
      {
        Effect = "Allow",
        Principal = { Service = "ecs-tasks.amazonaws.com" },
        Action = "sts:AssumeRole"
      }
    ]
  })
}

resource "aws_iam_role" "lambda_execution_role" {
  name = "lambda-execution-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17",
    Statement = [
      {
        Effect = "Allow",
        Principal = { Service = "lambda.amazonaws.com" },
        Action = "sts:AssumeRole"
      }
    ]
  })

  managed_policy_arns = [
    "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
  ]
}

output "cdn_bucket_name" {
  value = aws_s3_bucket.cdn_bucket.bucket
}

output "runtime_files_bucket_name" {
  value = aws_s3_bucket.runtime_files_bucket.bucket
}

output "ecs_cluster_name" {
  value = aws_ecs_cluster.fargate_cluster.name
}

output "lambda_function_name" {
  value = aws_lambda_function.lambda_function.function_name
}
