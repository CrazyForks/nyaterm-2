import { describe, expect, it } from "vitest";
import type { QuickCommand, QuickCommandCategory } from "@/types/global";
import {
  buildQuickCommandCategoryPath,
  buildQuickCommandCategoryTree,
  collectQuickCommandCategoryDescendantIds,
  deleteQuickCommandCategoryTree,
  hasQuickCommandCategorySiblingName,
} from "./quickCommandCategories";

describe("quickCommandCategories", () => {
  it("builds a tree with aggregate counts and orphan fallback", () => {
    const categories = [
      category("root", "Root"),
      category("child", "Child", "root"),
      category("orphan", "Orphan", "missing"),
    ];
    const commands = [
      command("cmd-root", "root"),
      command("cmd-child", "child"),
      command("cmd-orphan", "orphan"),
      command("cmd-missing", "missing-command-category"),
    ];

    const tree = buildQuickCommandCategoryTree(categories, commands);

    expect(tree.map((node) => node.category.id)).toEqual([
      "missing-command-category",
      "orphan",
      "root",
    ]);
    expect(tree.find((node) => node.category.id === "root")?.totalCount).toBe(
      2,
    );
    expect(
      tree.find((node) => node.category.id === "root")?.children[0].category.id,
    ).toBe("child");
    expect(tree.find((node) => node.category.id === "orphan")?.totalCount).toBe(
      1,
    );
  });

  it("collects descendants including the selected category", () => {
    const categories = [
      category("root", "Root"),
      category("child", "Child", "root"),
      category("nested", "Nested", "child"),
      category("other", "Other"),
    ];

    expect(
      Array.from(
        collectQuickCommandCategoryDescendantIds(categories, "root"),
      ).sort(),
    ).toEqual(["child", "nested", "root"]);
  });

  it("builds display paths", () => {
    const categories = [
      category("root", "Root"),
      category("child", "Child", "root"),
      category("nested", "Nested", "child"),
    ];

    expect(buildQuickCommandCategoryPath(categories, "nested")).toBe(
      "Root / Child / Nested",
    );
  });

  it("checks duplicate names by sibling scope", () => {
    const categories = [
      category("root", "Root"),
      category("child-a", "Deploy", "root"),
      category("child-b", "Deploy"),
    ];

    expect(
      hasQuickCommandCategorySiblingName(categories, "root", "deploy"),
    ).toBe(true);
    expect(hasQuickCommandCategorySiblingName(categories, null, "deploy")).toBe(
      true,
    );
    expect(
      hasQuickCommandCategorySiblingName(categories, "child-a", "deploy"),
    ).toBe(false);
  });

  it("deletes a category subtree with commands", () => {
    const categories = [
      category("root", "Root"),
      category("child", "Child", "root"),
      category("other", "Other"),
    ];
    const commands = [
      command("cmd-root", "root"),
      command("cmd-child", "child"),
      command("cmd-other", "other"),
      command("cmd-none"),
    ];

    const result = deleteQuickCommandCategoryTree(categories, commands, "root");

    expect(Array.from(result.deleteIds).sort()).toEqual(["child", "root"]);
    expect(result.categories.map((item) => item.id)).toEqual(["other"]);
    expect(result.commands.map((item) => item.id)).toEqual([
      "cmd-other",
      "cmd-none",
    ]);
  });
});

function category(
  id: string,
  name: string,
  parentId?: string,
): QuickCommandCategory {
  return { id, name, parent_id: parentId };
}

function command(id: string, categoryId?: string): QuickCommand {
  return {
    id,
    label: id,
    command: "echo test",
    category_id: categoryId,
  };
}
