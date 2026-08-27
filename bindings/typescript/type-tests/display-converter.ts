import {
  linguaToChatCompletionsDisplayMessages,
  type Message,
} from "../src";

declare const messages: Message[];

const displayMessages = linguaToChatCompletionsDisplayMessages(messages);
const reasoning: string | undefined = displayMessages[0]?.reasoning;

void reasoning;
