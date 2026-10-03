--!strict
-- A tomahawk strike (the original's attack object #0x6A, `sub_80D1A08`):
-- TomahawkMan's ETomahwk falling on a panel. After its delay it hits its
-- panel (48 pixels up) for 10 ticks with a burst of dust, and the panel
-- breaks (cracks while something stands on it). It waits while dimmed; a
-- hit ends its region. See docs/engine/standard-chips.md, "Action 0x0A".

local regions = require("../../lib/regions")

local strike = {}

-- What it is spawned with: its delay (Param1), collision types (Param2,
-- Param3) and spark (Param4; none: no spark).
export type Spec = { delay: number, is: CollisionType, hits: CollisionType, spark: Spark? }
export type State = Spec

local HEIGHT = 0x30_0000
local DUST = define.effect { sprite = asset.sprite("bn6:dust-2"), anim = 0, palette = 0 }
-- Its hit lasts 15 less 5 ticks.
local HIT_TICKS = 0x0F - 5
local HIT_MOD = 3
local WAIT, STRIKE = 0x0, 0x4

-- What it counts as having hit, and a guard (`sub_801A100`).
local HIT_SOMETHING, GUARDED = 0x3F80_0000, 1

-- `sub_801A100`: after a hit on something (not guarded), its spark over the
-- center of the panel it stands over, 16 pixels up, jittered.
local function panel_spark(me: Object)
    local f = me.collision.hit_flags
    local spark = (me.state :: State).spark
    if not bit32.btest(f, HIT_SOMETHING) or bit32.btest(f, GUARDED) or spark == nil then
        return
    end
    local px, py = int.tdiv(int.asr(me.pos.x, 16) + 0xA0, 0x28), int.tdiv(int.asr(me.pos.y, 16) + 0x20, 0x18)
    local cx, cy = field.center(px, py)
    battle.spark(me, battle.jitter(0x0F, Vec3.new(cx, cy, 0x10_0000)), spark)
end

-- `sub_80D1AB0`: its delay.
local function wait(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = (me.state :: State).delay
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    me.action = STRIKE
    me.phase = 0
    me.phase_init = 0
end

-- `sub_80D1AD2`: the hit, the dust and the panel; 10 ticks.
local function fall(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.collision:set_region(regions.single)
        local cx, cy = field.center(me.panel_x, me.panel_y)
        battle.effect(Vec3.new(cx, cy, 0), DUST, bit32.bxor(me.alliance, me.flip))
        field.break_panel(me.panel_x, me.panel_y)
        me.timer = HIT_TICKS
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80D1A64`.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    panel_spark(me)
    if battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    if not battle.dimmed() then
        local a = me.action
        if a == WAIT then
            wait(me)
        elseif a == STRIKE then
            fall(me)
        else
            error(string.format("tomahawk strike action %#x reads past off_80D1AA8", a))
        end
    end
    c:present()
end

-- `sub_80D1A28`: over its panel, its region off until it strikes; then
-- its first tick.
local function init(me: Object)
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, HEIGHT)
    if not me:create_collision() then
        me:free()
        return
    end
    local s = me.state :: State
    me:setup_collision(s.is, s.hits, HIT_MOD)
    me.collision:set_hit_effect(s.spark)
    me.collision:set_region(nil)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

strike.kind = define.kind {
    id = "bn6:tomahawkman/strike",
    pool = "attack",
    state = { delay = "u8", is = "collision", hits = "collision", spark = "spark" },
    -- `sub_80D1A08`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- `sub_80D1B22`: a strike on (x, y) from `owner`, with `element` and the
-- damage word `damage`. It runs while dimmed (and waits).
function strike.spawn(owner: Object, x: number, y: number, element: number, spec: Spec, damage: number): Object?
    local o = battle.spawn(strike.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
        local s = o.state :: State
        s.delay, s.is, s.hits, s.spark = spec.delay, spec.is, spec.hits, spec.spark
    end
    return o
end

return strike
