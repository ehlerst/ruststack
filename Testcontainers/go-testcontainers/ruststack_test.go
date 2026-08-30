package main

import (
	"context"
	"io"
	"strings"
	"testing"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/credentials"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb"
	dynamodbtypes "github.com/aws/aws-sdk-go-v2/service/dynamodb/types"
	"github.com/aws/aws-sdk-go-v2/service/s3"
	"github.com/aws/aws-sdk-go-v2/service/sqs"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/wait"
)

func startRustStackContainer(ctx context.Context, t *testing.T) (testcontainers.Container, string) {
	req := testcontainers.ContainerRequest{
		Image:        "ehlers320/ruststack:latest",
		ExposedPorts: []string{"4566/tcp"},
		WaitingFor:   wait.ForHTTP("/_ruststack/health").WithPort("4566/tcp").WithStartupTimeout(30 * time.Second),
	}

	container, err := testcontainers.GenericContainer(ctx, testcontainers.GenericContainerRequest{
		ContainerRequest: req,
		Started:          true,
	})
	if err != nil {
		t.Fatalf("Failed to start RustStack container: %v", err)
	}

	endpoint, err := container.Endpoint(ctx, "")
	if err != nil {
		t.Fatalf("Failed to get container endpoint: %v", err)
	}

	return container, "http://" + endpoint
}

func getAWSConfig(ctx context.Context, endpoint string) aws.Config {
	customResolver := aws.EndpointResolverWithOptionsFunc(func(service, region string, options ...interface{}) (aws.Endpoint, error) {
		return aws.Endpoint{
			URL:               endpoint,
			HostnameImmutable: true,
			SigningRegion:     "us-east-1",
		}, nil
	})

	cfg, _ := config.LoadDefaultConfig(ctx,
		config.WithRegion("us-east-1"),
		config.WithCredentialsProvider(credentials.NewStaticCredentialsProvider("test", "test", "")),
		config.WithEndpointResolverWithOptions(customResolver),
	)
	return cfg
}

func TestRustStackS3Integration(t *testing.T) {
	ctx := context.Background()
	container, endpoint := startRustStackContainer(ctx, t)
	defer container.Terminate(ctx)

	cfg := getAWSConfig(ctx, endpoint)
	s3Client := s3.NewFromConfig(cfg, func(o *s3.Options) {
		o.UsePathStyle = true
	})

	bucketName := "test-go-bucket"
	_, err := s3Client.CreateBucket(ctx, &s3.CreateBucketInput{
		Bucket: aws.String(bucketName),
	})
	if err != nil {
		t.Fatalf("CreateBucket failed: %v", err)
	}

	key := "hello.txt"
	content := "Hello from Go Testcontainers and RustStack!"
	_, err = s3Client.PutObject(ctx, &s3.PutObjectInput{
		Bucket: aws.String(bucketName),
		Key:    aws.String(key),
		Body:   strings.NewReader(content),
	})
	if err != nil {
		t.Fatalf("PutObject failed: %v", err)
	}

	getResp, err := s3Client.GetObject(ctx, &s3.GetObjectInput{
		Bucket: aws.String(bucketName),
		Key:    aws.String(key),
	})
	if err != nil {
		t.Fatalf("GetObject failed: %v", err)
	}
	defer getResp.Body.Close()

	bodyBytes, _ := io.ReadAll(getResp.Body)
	if string(bodyBytes) != content {
		t.Fatalf("Expected content %q, got %q", content, string(bodyBytes))
	}
}

func TestRustStackSQSIntegration(t *testing.T) {
	ctx := context.Background()
	container, endpoint := startRustStackContainer(ctx, t)
	defer container.Terminate(ctx)

	cfg := getAWSConfig(ctx, endpoint)
	sqsClient := sqs.NewFromConfig(cfg)

	createResp, err := sqsClient.CreateQueue(ctx, &sqs.CreateQueueInput{
		QueueName: aws.String("test-go-queue"),
	})
	if err != nil {
		t.Fatalf("CreateQueue failed: %v", err)
	}

	qUrl := createResp.QueueUrl

	msgBody := "Order payload #98765"
	_, err = sqsClient.SendMessage(ctx, &sqs.SendMessageInput{
		QueueUrl:    qUrl,
		MessageBody: aws.String(msgBody),
	})
	if err != nil {
		t.Fatalf("SendMessage failed: %v", err)
	}

	recvResp, err := sqsClient.ReceiveMessage(ctx, &sqs.ReceiveMessageInput{
		QueueUrl:            qUrl,
		MaxNumberOfMessages: 10,
	})
	if err != nil {
		t.Fatalf("ReceiveMessage failed: %v", err)
	}

	if len(recvResp.Messages) != 1 {
		t.Fatalf("Expected 1 message, got %d", len(recvResp.Messages))
	}

	if *recvResp.Messages[0].Body != msgBody {
		t.Fatalf("Expected body %q, got %q", msgBody, *recvResp.Messages[0].Body)
	}
}

func TestRustStackDynamoDBIntegration(t *testing.T) {
	ctx := context.Background()
	container, endpoint := startRustStackContainer(ctx, t)
	defer container.Terminate(ctx)

	cfg := getAWSConfig(ctx, endpoint)
	dynamoClient := dynamodb.NewFromConfig(cfg)

	tableName := "UsersTable"
	_, err := dynamoClient.CreateTable(ctx, &dynamodb.CreateTableInput{
		TableName: aws.String(tableName),
		AttributeDefinitions: []dynamodbtypes.AttributeDefinition{
			{
				AttributeName: aws.String("id"),
				AttributeType: dynamodbtypes.ScalarAttributeTypeS,
			},
		},
		KeySchema: []dynamodbtypes.KeySchemaElement{
			{
				AttributeName: aws.String("id"),
				KeyType:       dynamodbtypes.KeyTypeHash,
			},
		},
		BillingMode: dynamodbtypes.BillingModePayPerRequest,
	})
	if err != nil {
		t.Fatalf("CreateTable failed: %v", err)
	}

	_, err = dynamoClient.PutItem(ctx, &dynamodb.PutItemInput{
		TableName: aws.String(tableName),
		Item: map[string]dynamodbtypes.AttributeValue{
			"id":   &dynamodbtypes.AttributeValueMemberS{Value: "user-100"},
			"name": &dynamodbtypes.AttributeValueMemberS{Value: "Alice in Go"},
		},
	})
	if err != nil {
		t.Fatalf("PutItem failed: %v", err)
	}

	getResp, err := dynamoClient.GetItem(ctx, &dynamodb.GetItemInput{
		TableName: aws.String(tableName),
		Key: map[string]dynamodbtypes.AttributeValue{
			"id": &dynamodbtypes.AttributeValueMemberS{Value: "user-100"},
		},
	})
	if err != nil {
		t.Fatalf("GetItem failed: %v", err)
	}

	if getResp.Item == nil {
		t.Fatalf("Expected item, got nil")
	}

	nameVal, ok := getResp.Item["name"].(*dynamodbtypes.AttributeValueMemberS)
	if !ok || nameVal.Value != "Alice in Go" {
		t.Fatalf("Expected name 'Alice in Go', got %v", getResp.Item["name"])
	}
}
