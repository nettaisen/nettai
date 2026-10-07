-- Types shared across the exe4 pack's modules. A definitions file like
-- core.d.luau (luau-lsp: `.zed/generate-luau-definitions.py`; see README.md): types
-- a module and the modules that require it both name, declared once for
-- the pack. (The engine's API is in content/nettai/core.d.luau.)

-- What a patch card's effect works on in the reload besides the stats (rules/patch_cards): the HP the NaviCust's
-- programs and the cards add.
export type PatchCardWork = { hp: number }

-- The NaviCust's bug counts (rules/navicust's), which a card raises or clears.
export type PatchCardCounts = {
    add: (bug: string, n: number) -> (),
    clear: (bug: string) -> (),
}
