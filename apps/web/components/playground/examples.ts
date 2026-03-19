export const examples = [
  {
    name: "Basic Web Service",
    dsl: `title: Web Service
direction: LR

Client >> Load Balancer >> API Server 1 >> PostgreSQL
Load Balancer >> API Server 2 >> PostgreSQL
API Server 1 >> Redis
API Server 2 >> Redis

cluster AWS VPC {
  Load Balancer
  API Server 1
  API Server 2
  PostgreSQL
  Redis
}

cluster Application Tier {
  API Server 1
  API Server 2
}

cluster Data Tier {
  PostgreSQL
  Redis
}`,
  },
  {
    name: "AWS Architecture (Icons)",
    dsl: `title: AWS Web Architecture
direction: LR
use aws

aws:ELB Load Balancer >> aws:EC2 Web Server >> aws:RDS Database
aws:EC2 Web Server >> aws:S3 Static Assets
aws:EC2 Web Server >> aws:ElastiCache Cache

cluster:aws:region US East 1 {
  aws:ELB Load Balancer
  aws:EC2 Web Server
  aws:RDS Database
  aws:S3 Static Assets
  aws:ElastiCache Cache
}

cluster:aws:vpc Production VPC {
  aws:EC2 Web Server
  aws:RDS Database
  aws:ElastiCache Cache
}`,
  },
  {
    name: "GCP Architecture (Icons)",
    dsl: `title: GCP Data Pipeline
direction: LR
use gcp

gcp:cloud-storage Data Lake >> gcp:bigquery Analytics
gcp:compute-engine App Server >> gcp:cloud-sql Database
gcp:compute-engine App Server >> gcp:cloud-storage Data Lake
gcp:bigquery Analytics >> gcp:looker Dashboard
gcp:gke Microservices >> gcp:cloud-run Functions

cluster:gcp:region us-central1 {
  gcp:compute-engine App Server
  gcp:cloud-sql Database
  gcp:cloud-storage Data Lake
  gcp:bigquery Analytics
  gcp:looker Dashboard
  gcp:gke Microservices
  gcp:cloud-run Functions
}`,
  },
  {
    name: "Kubernetes (Icons)",
    dsl: `title: Kubernetes Microservices
direction: LR
use k8s

k8s:ingress Ingress >> k8s:service API Service >> k8s:deployment API Pods
k8s:deployment API Pods >> k8s:service DB Service >> k8s:stateful-set PostgreSQL
k8s:deployment API Pods >> k8s:service Cache Service >> k8s:deployment Redis
k8s:deployment API Pods >> k8s:secret Secrets

cluster:k8s:cluster Production Cluster {
  k8s:ingress Ingress
  k8s:service API Service
  k8s:deployment API Pods
  k8s:service DB Service
  k8s:stateful-set PostgreSQL
  k8s:service Cache Service
  k8s:deployment Redis
  k8s:secret Secrets
}`,
  },
  {
    name: "Microservices",
    dsl: `title: Microservices Architecture
direction: LR

Mobile App >> API Gateway
Web App >> API Gateway
API Gateway >> Auth
API Gateway >> User
API Gateway >> Order
Order >> Payment
Order >> Kafka
Auth >> Redis
User >> PostgreSQL
Order >> PostgreSQL
Payment >> MongoDB

cluster Services {
  Auth
  User
  Order
  Payment
}

cluster Storage {
  PostgreSQL
  MongoDB
  Redis
}`,
  },
  {
    name: "CI/CD Pipeline",
    dsl: `title: CI/CD Pipeline
direction: TB

Developer >> GitHub >> Build >> Test >> Security Scan
Security Scan >> Container Registry >> Staging >> Production >> Monitoring

cluster CI Pipeline {
  GitHub
  Build
  Test
  Security Scan
}

cluster CD Pipeline {
  Container Registry
  Staging
  Production
}`,
  },
  {
    name: "Event-Driven",
    dsl: `title: Event-Driven Architecture
direction: LR

Web API >> Event Bus
Mobile API >> Event Bus
IoT Devices >> Event Bus
Event Bus >> Notification
Event Bus >> Analytics
Event Bus >> Billing
Event Bus >> Search Index

cluster Event Producers {
  Web API
  Mobile API
  IoT Devices
}

cluster Event Consumers {
  Notification
  Analytics
  Billing
  Search Index
}`,
  },
];
