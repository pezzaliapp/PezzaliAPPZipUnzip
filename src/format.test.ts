import { describe, expect, it } from "vitest";
import {
  baseName, dirName, formatBytes, formatLabel, isArchiveName, isAscii, isValidName,
  passwordStrength, suggestArchiveName,
} from "./format";

describe("percorsi", () => {
  it("baseName e dirName gestiscono Unix e Windows", () => {
    expect(baseName("/Users/a/foto.zip")).toBe("foto.zip");
    expect(baseName("C:\\Dati\\foto.zip")).toBe("foto.zip");
    expect(baseName("/Users/a/cartella/")).toBe("cartella");
    expect(dirName("/Users/a/foto.zip")).toBe("/Users/a");
    expect(dirName("C:\\Dati\\foto.zip")).toBe("C:\\Dati");
    expect(dirName("C:\\foto.zip")).toBe("C:\\");
    expect(dirName("/foto.zip")).toBe("/");
  });

  it("riconosce gli archivi", () => {
    expect(isArchiveName("Backup.TAR.GZ")).toBe(true);
    expect(isArchiveName("film.part1.rar")).toBe(true);
    expect(isArchiveName("dati.7z.001")).toBe(true);
    expect(isArchiveName("documento.pdf")).toBe(false);
  });
});

describe("formattazione", () => {
  it("dimensioni leggibili", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1536, "en")).toBe("1.5 KB");
    expect(formatBytes(5 * 1024 * 1024, "en")).toBe("5.0 MB");
    expect(formatBytes(-1)).toBe("—");
  });

  it("nomi suggeriti", () => {
    expect(suggestArchiveName(["/a/b/Relazione.docx"])).toBe("Relazione");
    expect(suggestArchiveName(["/a/Progetto"])).toBe("Progetto");
    expect(suggestArchiveName(["/a/Vacanze/1.jpg", "/a/Vacanze/2.jpg"])).toBe("Vacanze");
    expect(suggestArchiveName([])).toBe("Archivio");
    expect(formatLabel("tar-gz")).toBe("TAR.GZ");
  });
});

describe("validazione", () => {
  it("password", () => {
    expect(passwordStrength("")).toBe(0);
    expect(passwordStrength("abc")).toBe(0);
    expect(passwordStrength("Abcdefgh1!")).toBe(3);
    expect(passwordStrength("Una Frase Lunga 2026!")).toBe(4);
    expect(isAscii("Passw0rd!")).toBe(true);
    expect(isAscii("Pàssword")).toBe(false);
  });

  it("nomi di file", () => {
    expect(isValidName("Archivio 2026")).toBe(true);
    expect(isValidName("../evil")).toBe(false);
    expect(isValidName("a:b")).toBe(false);
    expect(isValidName("  ")).toBe(false);
  });
});
