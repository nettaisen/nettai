-- Types shared across the bn6 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `--definitions=content/bn6/types.d.luau`): types
-- a module and the modules that require it both name, declared once for
-- the pack. (The engine's API and data types are in core.d.luau.)

-- Where an owner keeps an attached object: its actor overlay slot, its
-- first related object, or (a content kind) its state's `attachment`
-- field (lib/slot.luau).
export type Slot = "overlay" | "related" | "state"
