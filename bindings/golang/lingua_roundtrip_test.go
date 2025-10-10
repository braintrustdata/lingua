package lingua

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestSnapshot represents a test case loaded from the snapshots directory
type TestSnapshot struct {
	Name              string
	Provider          string // "chat-completions", "responses", or "anthropic"
	Turn              string // "first_turn" or "followup_turn"
	Request           map[string]interface{}
	Response          map[string]interface{}
	StreamingResponse []map[string]interface{}
}

// loadTestSnapshots loads all snapshots for a given test case
func loadTestSnapshots(t *testing.T, testCaseName string) []TestSnapshot {
	t.Helper()

	snapshots := []TestSnapshot{}
	snapshotsDir := filepath.Join("../../payloads/snapshots", testCaseName)

	providers := []string{"chat-completions", "responses", "anthropic"}
	turns := []struct {
		name   string
		prefix string
	}{
		{"first_turn", ""},
		{"followup_turn", "followup-"},
	}

	for _, provider := range providers {
		providerDir := filepath.Join(snapshotsDir, provider)

		// Check if provider directory exists
		if _, err := os.Stat(providerDir); os.IsNotExist(err) {
			continue
		}

		for _, turn := range turns {
			snapshot := TestSnapshot{
				Name:     testCaseName,
				Provider: provider,
				Turn:     turn.name,
			}

			// Load request
			requestPath := filepath.Join(providerDir, turn.prefix+"request.json")
			if data, err := os.ReadFile(requestPath); err == nil {
				var req map[string]interface{}
				if err := json.Unmarshal(data, &req); err == nil {
					snapshot.Request = req
				}
			}

			// Load response
			responsePath := filepath.Join(providerDir, turn.prefix+"response.json")
			if data, err := os.ReadFile(responsePath); err == nil {
				var resp map[string]interface{}
				if err := json.Unmarshal(data, &resp); err == nil {
					snapshot.Response = resp
				}
			}

			// Load streaming response
			streamingPath := filepath.Join(providerDir, turn.prefix+"response-streaming.json")
			if data, err := os.ReadFile(streamingPath); err == nil {
				// Try parsing as JSON array first
				var streamResp []map[string]interface{}
				if err := json.Unmarshal(data, &streamResp); err == nil {
					snapshot.StreamingResponse = streamResp
				} else {
					// Try newline-delimited JSON
					lines := strings.Split(string(data), "\n")
					for _, line := range lines {
						line = strings.TrimSpace(line)
						if line == "" {
							continue
						}
						var item map[string]interface{}
						if err := json.Unmarshal([]byte(line), &item); err == nil {
							snapshot.StreamingResponse = append(snapshot.StreamingResponse, item)
						}
					}
				}
			}

			// Only add snapshot if it has at least one payload
			if snapshot.Request != nil || snapshot.Response != nil || len(snapshot.StreamingResponse) > 0 {
				snapshots = append(snapshots, snapshot)
			}
		}
	}

	return snapshots
}

// normalizeForComparison recursively normalizes an object by removing null, undefined, and empty values
// This mimics how Rust's serde skips None values during serialization
func normalizeForComparison(obj interface{}) interface{} {
	if obj == nil {
		return nil
	}

	switch v := obj.(type) {
	case map[string]interface{}:
		normalized := make(map[string]interface{})
		for key, value := range v {
			normalizedValue := normalizeForComparison(value)
			// Only include non-nil values and non-empty maps/arrays
			if normalizedValue != nil {
				if m, ok := normalizedValue.(map[string]interface{}); ok && len(m) == 0 {
					continue
				}
				if a, ok := normalizedValue.([]interface{}); ok && len(a) == 0 {
					continue
				}
				normalized[key] = normalizedValue
			}
		}
		if len(normalized) == 0 {
			return nil
		}
		return normalized

	case []interface{}:
		normalized := []interface{}{}
		for _, item := range v {
			normalizedItem := normalizeForComparison(item)
			if normalizedItem != nil {
				normalized = append(normalized, normalizedItem)
			}
		}
		if len(normalized) == 0 {
			return nil
		}
		return normalized

	default:
		return v
	}
}

// deepEqual checks if two objects are deeply equal after normalization
func deepEqual(a, b interface{}) bool {
	normalizedA := normalizeForComparison(a)
	normalizedB := normalizeForComparison(b)
	return reflect.DeepEqual(normalizedA, normalizedB)
}

// TestChatCompletionsRoundtrip tests roundtrip conversion for OpenAI Chat Completions format
func TestChatCompletionsRoundtrip(t *testing.T) {
	snapshotsDir := "../../payloads/snapshots"

	// Get all test cases
	entries, err := os.ReadDir(snapshotsDir)
	if err != nil {
		t.Fatalf("Failed to read snapshots directory: %v", err)
	}

	testCases := []string{}
	for _, entry := range entries {
		if entry.IsDir() && !strings.HasPrefix(entry.Name(), ".") {
			testCases = append(testCases, entry.Name())
		}
	}

	require.NotEmpty(t, testCases, "No test cases found in snapshots directory")

	for _, testCase := range testCases {
		t.Run(testCase, func(t *testing.T) {
			snapshots := loadTestSnapshots(t, testCase)

			if len(snapshots) == 0 {
				t.Skip("No snapshots found for this test case")
				return
			}

			for _, snapshot := range snapshots {
				if snapshot.Provider != "chat-completions" || snapshot.Request == nil {
					continue
				}

				testName := snapshot.Provider + " - " + snapshot.Turn
				t.Run(testName, func(t *testing.T) {
					messages, ok := snapshot.Request["messages"].([]interface{})
					require.True(t, ok, "Request should have messages array")
					require.NotEmpty(t, messages, "Messages array should not be empty")

					// Test each message in the request
					for i, msgInterface := range messages {
						originalMessage, ok := msgInterface.(map[string]interface{})
						require.True(t, ok, "Message should be a map")

						t.Run("message_"+string(rune(i)), func(t *testing.T) {
							// Perform the roundtrip: Chat Completions -> Lingua -> Chat Completions
							// Convert to Lingua
							linguaMessages, err := ChatCompletionsMessagesToLingua([]interface{}{originalMessage})
							require.NoError(t, err, "Failed to convert to Lingua format")
							require.Len(t, linguaMessages, 1, "Should have exactly one Lingua message")

							linguaMessage := linguaMessages[0]
							assert.NotNil(t, linguaMessage["role"], "Lingua message should have role")

							// Convert back to Chat Completions
							roundtrippedMessages, err := LinguaToChatCompletionsMessages(linguaMessages)
							require.NoError(t, err, "Failed to convert back to Chat Completions format")
							require.Len(t, roundtrippedMessages, 1, "Should have exactly one roundtripped message")

							roundtrippedMessage := roundtrippedMessages[0]

							// Verify the roundtrip preserved the data
							// Normalize both to handle serde's None-skipping behavior
							if !deepEqual(originalMessage, roundtrippedMessage) {
								// Pretty print for debugging
								originalPretty, _ := json.MarshalIndent(originalMessage, "", "  ")
								roundtrippedPretty, _ := json.MarshalIndent(roundtrippedMessage, "", "  ")
								t.Errorf("Roundtrip did not preserve data:\nOriginal:\n%s\n\nRoundtripped:\n%s",
									string(originalPretty), string(roundtrippedPretty))
							}
						})
					}
				})
			}
		})
	}
}

// TestAnthropicRoundtrip tests roundtrip conversion for Anthropic format
func TestAnthropicRoundtrip(t *testing.T) {
	snapshotsDir := "../../payloads/snapshots"

	// Get all test cases
	entries, err := os.ReadDir(snapshotsDir)
	if err != nil {
		t.Fatalf("Failed to read snapshots directory: %v", err)
	}

	testCases := []string{}
	for _, entry := range entries {
		if entry.IsDir() && !strings.HasPrefix(entry.Name(), ".") {
			testCases = append(testCases, entry.Name())
		}
	}

	require.NotEmpty(t, testCases, "No test cases found in snapshots directory")

	for _, testCase := range testCases {
		t.Run(testCase, func(t *testing.T) {
			snapshots := loadTestSnapshots(t, testCase)

			if len(snapshots) == 0 {
				t.Skip("No snapshots found for this test case")
				return
			}

			for _, snapshot := range snapshots {
				if snapshot.Provider != "anthropic" || snapshot.Request == nil {
					continue
				}

				testName := snapshot.Provider + " - " + snapshot.Turn
				t.Run(testName, func(t *testing.T) {
					messages, ok := snapshot.Request["messages"].([]interface{})
					require.True(t, ok, "Request should have messages array")
					require.NotEmpty(t, messages, "Messages array should not be empty")

					// Test each message in the request
					for i, msgInterface := range messages {
						originalMessage, ok := msgInterface.(map[string]interface{})
						require.True(t, ok, "Message should be a map")

						t.Run("message_"+string(rune(i)), func(t *testing.T) {
							// Perform the roundtrip: Anthropic -> Lingua -> Anthropic
							// Convert to Lingua
							linguaMessages, err := AnthropicMessagesToLingua([]interface{}{originalMessage})
							require.NoError(t, err, "Failed to convert to Lingua format")
							require.Len(t, linguaMessages, 1, "Should have exactly one Lingua message")

							linguaMessage := linguaMessages[0]
							assert.NotNil(t, linguaMessage["role"], "Lingua message should have role")

							// Convert back to Anthropic
							roundtrippedMessages, err := LinguaToAnthropicMessages(linguaMessages)
							require.NoError(t, err, "Failed to convert back to Anthropic format")
							require.Len(t, roundtrippedMessages, 1, "Should have exactly one roundtripped message")

							roundtrippedMessage := roundtrippedMessages[0]

							// Verify the roundtrip preserved the data
							// Normalize both to handle serde's None-skipping behavior
							if !deepEqual(originalMessage, roundtrippedMessage) {
								// Pretty print for debugging
								originalPretty, _ := json.MarshalIndent(originalMessage, "", "  ")
								roundtrippedPretty, _ := json.MarshalIndent(roundtrippedMessage, "", "  ")
								t.Errorf("Roundtrip did not preserve data:\nOriginal:\n%s\n\nRoundtripped:\n%s",
									string(originalPretty), string(roundtrippedPretty))
							}
						})
					}
				})
			}
		})
	}
}

// TestSnapshotCoverage verifies that we have good test coverage across all snapshot cases
func TestSnapshotCoverage(t *testing.T) {
	snapshotsDir := "../../payloads/snapshots"

	// Get all test cases
	entries, err := os.ReadDir(snapshotsDir)
	require.NoError(t, err, "Failed to read snapshots directory")

	testCases := []string{}
	for _, entry := range entries {
		if entry.IsDir() && !strings.HasPrefix(entry.Name(), ".") {
			testCases = append(testCases, entry.Name())
		}
	}

	require.NotEmpty(t, testCases, "No test cases found in snapshots directory")

	coverage := make(map[string]struct {
		Providers []string
		Turns     []string
	})

	for _, testCase := range testCases {
		snapshots := loadTestSnapshots(t, testCase)

		providers := make(map[string]bool)
		turns := make(map[string]bool)

		for _, snapshot := range snapshots {
			providers[snapshot.Provider] = true
			turns[snapshot.Turn] = true
		}

		providerList := []string{}
		for p := range providers {
			providerList = append(providerList, p)
		}

		turnList := []string{}
		for tr := range turns {
			turnList = append(turnList, tr)
		}

		coverage[testCase] = struct {
			Providers []string
			Turns     []string
		}{
			Providers: providerList,
			Turns:     turnList,
		}
	}

	t.Log("Test coverage by case:")
	for testCase, data := range coverage {
		t.Logf("  %s:", testCase)
		t.Logf("    Providers: %s", strings.Join(data.Providers, ", "))
		t.Logf("    Turns: %s", strings.Join(data.Turns, ", "))

		// Ensure each test case has at least some snapshots
		assert.NotEmpty(t, data.Providers, "Test case %s should have at least one provider", testCase)
	}
}
