const unsupportedForcedToolChoice = new RegExp(
  "(^|[./:@]|databricks-)claude-((opus|sonnet)-5[-.]5|(fable|mythos)-5[-.][12])($|[-./:@])"
);

export function supportsForcedToolChoice(model: string): boolean {
  return !unsupportedForcedToolChoice.test(
    model.replace(/[A-Z]/g, (character) => character.toLowerCase())
  );
}
