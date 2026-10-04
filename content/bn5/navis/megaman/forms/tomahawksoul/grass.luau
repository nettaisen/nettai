--!strict
-- TomahawkSoul's grass (BN5's effect object 0x16, 0x080E1FC0: BN6's `sub_80E1E4C`, the same code, which BN6's
-- netbattles never reach), spawned by the status reset in TomahawkSoul (0x08011C98, running while paused and
-- dimmed): for its ticks every panel on the field shows as grass, two ticks of four (`object_setPanelTypeBlink`,
-- keeping its side), then every panel turns to grass and it goes. Its parameters name the type and the ticks: the
-- reset's are grass and 10. (Its other modes, Param2 1 to 3: the other side's area, the area ahead, the middle row
-- and four panels, which turn cracked or to the type, have no caller.)

local grass = {}

-- Its parameters (0x000A0006): Param1 the panel type, Param3 the ticks it shows.
local KIND: PanelType, TICKS = "grass", 10

-- 0x080E1FE4: the count up; showing the field as the type two ticks of four; at the end the field turned, and gone.
local function change(me: Object)
    local t = me.timer + 1
    me.timer = t
    if t >= TICKS then
        for y = 3, 1, -1 do
            for x = 6, 1, -1 do
                field.set_type(x, y, KIND)
            end
        end
        me:free()
        return
    end
    if bit32.btest(t, 4) then
        return
    end
    for y = 3, 1, -1 do
        for x = 6, 1, -1 do
            local p = field.panel(x, y)
            if p and p.kind ~= "missing" then
                field.blink(x, y, KIND, p.alliance)
            end
        end
    end
end

grass.kind = define.kind {
    id = "tomahawksoul/grass",
    pool = "effect",
    -- 0x080E1FC0 (by Param2: 0, the whole field).
    update = change,
}

-- 0x080E21DC: the grass for `navi`'s side, running while paused and dimmed (the reset's parameters 0x000A0006).
function grass.spawn(navi: Object): Object?
    local o = battle.spawn(grass.kind)
    if o then
        -- (Its panel is the spawner's r0 and r1, which the status reset's dispatch by soul (0x08011B92) leaves: the
        -- low bytes of this routine's address, 0x08011C99, and of the dispatch's table, 0x08011BAC.)
        o.panel_x, o.panel_y = 0x99, 0xAC
        o.alliance = navi.alliance
        o.run_while_paused = true
        o.run_while_dimmed = true
    end
    return o
end

return grass
