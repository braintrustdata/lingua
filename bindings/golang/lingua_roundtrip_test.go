package lingua

import (
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
)

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
