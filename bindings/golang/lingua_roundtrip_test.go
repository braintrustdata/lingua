package lingua

import (
	"encoding/json/jsontext"
	jsonv2 "encoding/json/v2"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestSnapshot represents a test case loaded from the snapshots directory.
type TestSnapshot struct {
	Name              string
	Provider          string // "chat-completions", "responses", or "anthropic"
	Turn              string // "first_turn" or "followup_turn"
	Request           map[string]any
	Response          map[string]any
	StreamingResponse []map[string]any
}

const snapshotsBase = "../../payloads/snapshots"

var (
	snapshotProviders = []string{"chat-completions", "responses", "anthropic"}
	snapshotTurns     = []struct {
		name   string
		prefix string
	}{
		{name: "first_turn", prefix: ""},
		{name: "followup_turn", prefix: "followup-"},
	}
)

func listSnapshotTestCases(t *testing.T) []string {
	t.Helper()

	entries, err := os.ReadDir(snapshotsBase)
	require.NoError(t, err, "Failed to read snapshots directory")

	testCases := []string{}
	for _, entry := range entries {
		if entry.IsDir() && !strings.HasPrefix(entry.Name(), ".") {
			testCases = append(testCases, entry.Name())
		}
	}

	require.NotEmpty(t, testCases, "No test cases found in snapshots directory")
	return testCases
}

// loadTestSnapshots loads all snapshots for a given test case.
func loadTestSnapshots(t *testing.T, testCaseName string) []TestSnapshot {
	t.Helper()

	snapshotsDir := filepath.Join(snapshotsBase, testCaseName)

	var snapshots []TestSnapshot
	for _, provider := range snapshotProviders {
		snapshots = append(snapshots, loadProviderSnapshots(testCaseName, provider, snapshotsDir)...)
	}

	return snapshots
}

func loadProviderSnapshots(testCaseName, provider, snapshotsDir string) []TestSnapshot {
	providerDir := filepath.Join(snapshotsDir, provider)
	info, err := os.Stat(providerDir)
	if err != nil || !info.IsDir() {
		return nil
	}

	var snapshots []TestSnapshot
	for _, turn := range snapshotTurns {
		snapshot := TestSnapshot{
			Name:     testCaseName,
			Provider: provider,
			Turn:     turn.name,
		}

		snapshot.Request = loadSnapshotMap(filepath.Join(providerDir, turn.prefix+"request.json"))
		snapshot.Response = loadSnapshotMap(filepath.Join(providerDir, turn.prefix+"response.json"))
		snapshot.StreamingResponse = loadStreamingSnapshot(filepath.Join(providerDir, turn.prefix+"response-streaming.json"))

		if snapshot.Request != nil || snapshot.Response != nil || len(snapshot.StreamingResponse) > 0 {
			snapshots = append(snapshots, snapshot)
		}
	}

	return snapshots
}

func loadSnapshotMap(path string) map[string]any {
	data, err := readSnapshotFile(path)
	if err != nil {
		return nil
	}

	var result map[string]any
	if err := jsonv2.Unmarshal(data, &result); err != nil {
		return nil
	}

	return result
}

func loadStreamingSnapshot(path string) []map[string]any {
	data, err := readSnapshotFile(path)
	if err != nil {
		return nil
	}

	var streamResp []map[string]any
	if err := jsonv2.Unmarshal(data, &streamResp); err == nil {
		return streamResp
	}

	lines := strings.Split(string(data), "\n")
	var items []map[string]any
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}

		var item map[string]any
		if err := jsonv2.Unmarshal([]byte(line), &item); err == nil {
			items = append(items, item)
		}
	}

	if len(items) == 0 {
		return nil
	}

	return items
}

func readSnapshotFile(path string) ([]byte, error) {
	// #nosec G304 -- reading trusted test fixture data from repository
	return os.ReadFile(path)
}

// normalizeForComparison recursively removes empty slices/maps to match Rust's serde behavior.
//
// This mimics how Rust's serde skips None values during serialization.
func normalizeForComparison(obj any) any {
	switch v := obj.(type) {
	case nil:
		return nil
	case map[string]any:
		return normalizeMapForComparison(v)
	case []any:
		return normalizeSliceForComparison(v)
	default:
		return v
	}
}

func normalizeMapForComparison(values map[string]any) map[string]any {
	normalized := make(map[string]any)
	for key, value := range values {
		normalizedValue := normalizeForComparison(value)
		if !shouldIncludeNormalizedValue(normalizedValue) {
			continue
		}
		normalized[key] = normalizedValue
	}
	if len(normalized) == 0 {
		return nil
	}
	return normalized
}

func normalizeSliceForComparison(items []any) []any {
	normalized := make([]any, 0, len(items))
	for _, item := range items {
		normalizedItem := normalizeForComparison(item)
		if !shouldIncludeNormalizedValue(normalizedItem) {
			continue
		}
		normalized = append(normalized, normalizedItem)
	}
	if len(normalized) == 0 {
		return nil
	}
	return normalized
}

func shouldIncludeNormalizedValue(value any) bool {
	if value == nil {
		return false
	}
	if normalizedMap, ok := value.(map[string]any); ok {
		return len(normalizedMap) > 0
	}
	if normalizedArray, ok := value.([]any); ok {
		return len(normalizedArray) > 0
	}
	return true
}

// deepEqual checks if two objects are deeply equal after normalization.
func deepEqual(a, b any) bool {
	normalizedA := normalizeForComparison(a)
	normalizedB := normalizeForComparison(b)
	return reflect.DeepEqual(normalizedA, normalizedB)
}

func runRoundtripTests(
	t *testing.T,
	provider string,
	toLingua func([]any) ([]map[string]any, error),
	fromLingua func([]map[string]any) ([]map[string]any, error),
) {
	t.Helper()

	for _, testCase := range listSnapshotTestCases(t) {
		testCase := testCase
		t.Run(testCase, func(t *testing.T) {
			runRoundtripTestCase(t, testCase, provider, toLingua, fromLingua)
		})
	}
}

func runRoundtripTestCase(
	t *testing.T,
	testCase string,
	provider string,
	toLingua func([]any) ([]map[string]any, error),
	fromLingua func([]map[string]any) ([]map[string]any, error),
) {
	t.Helper()

	snapshots := loadTestSnapshots(t, testCase)
	if len(snapshots) == 0 {
		t.Skip("No snapshots found for this test case")
		return
	}

	for i := range snapshots {
		snapshot := &snapshots[i]
		if !shouldRunSnapshot(provider, snapshot) {
			continue
		}

		t.Run(snapshot.Provider+" - "+snapshot.Turn, func(t *testing.T) {
			runRoundtripSnapshot(t, provider, snapshot, toLingua, fromLingua)
		})
	}
}

func shouldRunSnapshot(provider string, snapshot *TestSnapshot) bool {
	return snapshot.Provider == provider && snapshot.Request != nil
}

func runRoundtripSnapshot(
	t *testing.T,
	provider string,
	snapshot *TestSnapshot,
	toLingua func([]any) ([]map[string]any, error),
	fromLingua func([]map[string]any) ([]map[string]any, error),
) {
	t.Helper()

	field := snapshotRequestField(provider)
	messages := extractSnapshotMessages(t, snapshot, field)

	for index, msgInterface := range messages {
		index := index
		msgInterface := msgInterface
		t.Run(fmt.Sprintf("message_%d", index), func(t *testing.T) {
			runRoundtripMessage(t, msgInterface, toLingua, fromLingua)
		})
	}
}

func snapshotRequestField(provider string) string {
	if provider == "responses" {
		return "input"
	}
	return "messages"
}

func extractSnapshotMessages(t *testing.T, snapshot *TestSnapshot, field string) []any {
	t.Helper()

	messagesValue, ok := snapshot.Request[field]
	require.Truef(t, ok, "Request should have %s array", field)

	messages, ok := messagesValue.([]any)
	require.Truef(t, ok, "%s field should be an array", field)
	require.NotEmptyf(t, messages, "%s array should not be empty", field)

	return messages
}

func runRoundtripMessage(
	t *testing.T,
	msgInterface any,
	toLingua func([]any) ([]map[string]any, error),
	fromLingua func([]map[string]any) ([]map[string]any, error),
) {
	t.Helper()

	originalMessage, ok := msgInterface.(map[string]any)
	require.True(t, ok, "Message should be a map")

	linguaMessages := convertToLingua(t, originalMessage, toLingua)
	linguaMessage := linguaMessages[0]
	assert.NotNil(t, linguaMessage["role"], "Lingua message should have role")

	roundtrippedMessages := convertFromLingua(t, linguaMessages, fromLingua)
	roundtrippedMessage := roundtrippedMessages[0]

	assertRoundtripEquality(t, originalMessage, roundtrippedMessage, linguaMessage)
}

func convertToLingua(
	t *testing.T,
	originalMessage map[string]any,
	toLingua func([]any) ([]map[string]any, error),
) []map[string]any {
	t.Helper()

	linguaMessages, err := toLingua([]any{originalMessage})
	require.NoError(t, err, "Failed to convert to Lingua format")
	require.Len(t, linguaMessages, 1, "Should have exactly one Lingua message")
	return linguaMessages
}

func convertFromLingua(
	t *testing.T,
	linguaMessages []map[string]any,
	fromLingua func([]map[string]any) ([]map[string]any, error),
) []map[string]any {
	t.Helper()

	roundtrippedMessages, err := fromLingua(linguaMessages)
	require.NoError(t, err, "Failed to convert back to provider format")
	require.Len(t, roundtrippedMessages, 1, "Should have exactly one roundtripped message")
	return roundtrippedMessages
}

func assertRoundtripEquality(
	t *testing.T,
	originalMessage map[string]any,
	roundtrippedMessage map[string]any,
	linguaMessage map[string]any,
) {
	t.Helper()

	if deepEqual(originalMessage, roundtrippedMessage) {
		return
	}

	originalPretty := mustPrettyJSON(t, originalMessage)
	roundtrippedPretty := mustPrettyJSON(t, roundtrippedMessage)
	linguaPretty := mustPrettyJSON(t, linguaMessage)

	t.Fatalf(
		"Roundtrip mismatch:\nOriginal:\n%s\nRoundtripped:\n%s\nLingua intermediate:\n%s",
		originalPretty,
		roundtrippedPretty,
		linguaPretty,
	)
}

func mustPrettyJSON(t *testing.T, value any) string {
	t.Helper()

	data, err := jsonv2.Marshal(value, jsontext.WithIndent("  "))
	require.NoError(t, err, "Failed to pretty print value")
	return string(data)
}

// TestChatCompletionsRoundtrip tests roundtrip conversion for OpenAI Chat Completions format.
func TestChatCompletionsRoundtrip(t *testing.T) {
	runRoundtripTests(
		t,
		"chat-completions",
		func(messages []any) ([]map[string]any, error) {
			return ChatCompletionsMessagesToLingua(messages)
		},
		func(messages []map[string]any) ([]map[string]any, error) {
			return LinguaToChatCompletionsMessages(messages)
		},
	)
}

// TestAnthropicRoundtrip tests roundtrip conversion for Anthropic format.
func TestAnthropicRoundtrip(t *testing.T) {
	runRoundtripTests(
		t,
		"anthropic",
		func(messages []any) ([]map[string]any, error) {
			return AnthropicMessagesToLingua(messages)
		},
		func(messages []map[string]any) ([]map[string]any, error) {
			return LinguaToAnthropicMessages(messages)
		},
	)
}

// TestResponsesRoundtrip tests roundtrip conversion for OpenAI Responses API format.
func TestResponsesRoundtrip(t *testing.T) {
	runRoundtripTests(
		t,
		"responses",
		func(messages []any) ([]map[string]any, error) {
			return ResponsesMessagesToLingua(messages)
		},
		func(messages []map[string]any) ([]map[string]any, error) {
			return LinguaToResponsesMessages(messages)
		},
	)
}

// TestSnapshotCoverage verifies that we have good test coverage across all snapshot cases.
func TestSnapshotCoverage(t *testing.T) {
	testCases := listSnapshotTestCases(t)

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
