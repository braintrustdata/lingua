package main

import (
	"fmt"
	"log"

	"github.com/braintrustdata/lingua/bindings/golang"
)

func main() {
	fmt.Println("Lingua Go Bindings - Basic Conversion Example")
	fmt.Println("==============================================\n")

	// Example 1: Chat Completions to Lingua
	fmt.Println("1. Converting Chat Completions messages to Lingua format:")
	chatMsgs := []map[string]interface{}{
		{"role": "user", "content": "Hello, how are you?"},
		{"role": "assistant", "content": "I'm doing well, thank you! How can I help you today?"},
	}

	linguaMsgs, err := lingua.ChatCompletionsMessagesToLingua(chatMsgs)
	if err != nil {
		log.Fatalf("Conversion failed: %v", err)
	}
	fmt.Printf("Lingua messages: %+v\n\n", linguaMsgs)

	// Example 2: Lingua to Anthropic
	fmt.Println("2. Converting Lingua messages to Anthropic format:")
	anthropicMsgs, err := lingua.LinguaToAnthropicMessages(linguaMsgs)
	if err != nil {
		log.Fatalf("Conversion failed: %v", err)
	}
	fmt.Printf("Anthropic messages: %+v\n\n", anthropicMsgs)

	// Example 3: Cross-provider conversion (OpenAI -> Lingua -> Anthropic)
	fmt.Println("3. Cross-provider conversion (OpenAI -> Anthropic):")
	openaiMsgs := []map[string]interface{}{
		{"role": "user", "content": "What is the capital of France?"},
		{"role": "assistant", "content": "The capital of France is Paris."},
	}

	// OpenAI -> Lingua
	lingua2, err := lingua.ChatCompletionsMessagesToLingua(openaiMsgs)
	if err != nil {
		log.Fatalf("Conversion failed: %v", err)
	}

	// Lingua -> Anthropic
	anthropic2, err := lingua.LinguaToAnthropicMessages(lingua2)
	if err != nil {
		log.Fatalf("Conversion failed: %v", err)
	}
	fmt.Printf("Result: %+v\n\n", anthropic2)

	// Example 4: Deduplication
	fmt.Println("4. Deduplicating messages:")
	duplicateMsgs := []map[string]interface{}{
		{"role": "user", "content": "Hello"},
		{"role": "user", "content": "Hello"}, // Duplicate
		{"role": "assistant", "content": "Hi!"},
	}

	deduplicated, err := lingua.DeduplicateMessages(duplicateMsgs)
	if err != nil {
		log.Fatalf("Deduplication failed: %v", err)
	}
	fmt.Printf("Original count: %d, After deduplication: %d\n", len(duplicateMsgs), len(deduplicated))
	fmt.Printf("Deduplicated: %+v\n\n", deduplicated)

	// Example 5: Validation
	fmt.Println("5. Validating OpenAI request:")
	validRequest := `{
		"model": "gpt-4",
		"messages": [
			{"role": "user", "content": "Hello"}
		]
	}`

	validated, err := lingua.ValidateChatCompletionsRequest(validRequest)
	if err != nil {
		fmt.Printf("Validation failed: %v\n", err)
	} else {
		fmt.Printf("Valid request! Model: %s\n", validated["model"])
	}

	fmt.Println("\nAll examples completed successfully!")
}
