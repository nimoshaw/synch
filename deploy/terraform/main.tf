provider "aws" {
  region = "us-west-2"
}

# 这是一个基础架构示例，用于配置 ECS 或 EKS 运行 Relay Server
resource "aws_vpc" "synch_vpc" {
  cidr_block = "10.0.0.0/16"
  enable_dns_hostnames = true
  tags = {
    Name = "synch-vpc"
  }
}

# ECR 镜像仓库
resource "aws_ecr_repository" "synch_relay" {
  name                 = "synch/relay"
  image_tag_mutability = "MUTABLE"
  
  image_scanning_configuration {
    scan_on_push = true
  }
}

# Redis 集群 (ElastiCache)
resource "aws_elasticache_cluster" "synch_redis" {
  cluster_id           = "synch-redis-cluster"
  engine               = "redis"
  node_type            = "cache.t4g.micro"
  num_cache_nodes      = 1
  parameter_group_name = "default.redis7"
  engine_version       = "7.0"
  port                 = 6379
}
