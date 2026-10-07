--!strict
-- EXE4's light and dark MegaMan, as far as ported (docs/design/exe4-map.md §18 item 12): the save's light/dark value
-- (NaviStats +0x36, 0x0800F5A6), by which a navi that isn't dark closes the hole he stands on.
--
-- Not yet (exe4-map.md §18): what else reads the value (the mood, 0x0800D836; the dark chips).

export type Setup = {
    -- The save's light/dark value, its karma (NaviStats +0x36): 0 to 1000; the default block's 500 (0x0800D6BE),
    -- the setup's default. (A match's side's `karma`, which nettai-match writes by this field's name.)
    karma: number,
}

-- The value above which a navi closes a hole (0x08013348: `cmp r0, 0x1F3; ble`): a light one's, and the default
-- 500.
local CLOSES_ABOVE = 499

-- 0x08013318, each tick of the navi's intake: unless the battle is dimmed, a navi whose value is above 499 standing
-- on a hole (his panel's) mends the panel his body is on (his collision's) to normal. (Its check of 0x0800F49C counts
-- for nothing: the `movs r0, #5` before its `beq` sets the flags it tests. Nor does NaviStats +0x26's 2, which skips
-- it: only a story's other navi, 0x08041100, has it.)
local function navi_intake(_side: number, navi: Object)
    if battle.dimmed() then
        return
    end
    local p = field.panel(navi.panel_x, navi.panel_y)
    if p == nil or p.kind ~= "hole" then
        return
    end
    if (rules.setup() :: Setup).karma <= CLOSES_ABOVE then
        return
    end
    field.set_type(navi.collision.panel_x, navi.collision.panel_y, "normal")
end

return {
    navi_intake = navi_intake,
}
