--!strict
-- FireSoul's grass (EXE4's effect object 0x12, 0x080E33E8; spawned by 0x080E3604, which only FireSoul's status reset
-- calls, 0x0800E11C: its parameters 0x000A0306): unseen and never leaving its first state, it counts its ticks from its
-- spawn's own; while its count's bit 2 is clear the panels it changes show as grass (0x08009E38, drawn only), and at
-- 10 they turn grass and it goes. The panels (its mode 3, 0x080E3552): the middle row's, x 6 to 0, and four more,
-- the panels above and below each side's middle column (0x080E35FC); a missing panel stays. (The effect's modes 0 to 2,
-- every panel, the other side's and by a column, have no spawner that picks them.)

local grass = {}

local GRASS: PanelType = "grass"
local TICKS = 10
local ROW = 2
local MORE: { { number } } = { { 2, 1 }, { 2, 3 }, { 5, 1 }, { 5, 3 } }

-- Each panel it changes, in its order, that isn't missing.
local function each(f: (x: number, y: number, p: Panel) -> ())
    local function one(x: number, y: number)
        local p = field.panel(x, y)
        if p ~= nil and p.kind ~= "missing" then
            f(x, y, p)
        end
    end
    for x = 6, 0, -1 do
        one(x, ROW)
    end
    for _, xy in MORE do
        one(xy[1], xy[2])
    end
end

-- 0x080E3552.
local function tick(me: Object)
    local t = me.timer + 1
    me.timer = t
    if t >= TICKS then
        each(function(x: number, y: number, _p: Panel)
            field.set_type(x, y, GRASS)
        end)
        me:free()
        return
    end
    if not bit32.btest(t, 4) then
        each(function(x: number, y: number, p: Panel)
            field.blink(x, y, GRASS, p.alliance)
        end)
    end
end

grass.kind = new.kind {
    id = "firesoul/grass",
    pool = "effect",
    -- 0x080E33E8: by its mode alone, whatever its state.
    update = tick,
}

-- 0x080E3604: the grass of `navi`'s side (at the status reset's registers: its panel and position, which nothing
-- reads, code addresses of the reset's).
function grass.spawn(navi: Object): Object?
    local o = battle.spawn(grass.kind)
    if o then
        o.alliance = navi.alliance
    end
    return o
end

return grass
