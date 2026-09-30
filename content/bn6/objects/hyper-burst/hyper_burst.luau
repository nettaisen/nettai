--!strict
-- H-Burst's shot (attack object #0x9E, `sub_80D8FC4`): an unseen shot
-- stepping a panel a tick along its row (collision type 0x3D, status
-- 0x10) until it hits something. There it bursts Param1 | Param2 << 8
-- times (H-Burst: 10), every 9 ticks: a hit on the panel with region 9
-- and region 0xB in turn (collision types 5 and 4, status 0x10), with an
-- explosion (effect 0x3D, 10 pixels up) on each panel of the region. It
-- ends when it leaves the field, when the bursts are done, or when the
-- fight stops.

export type State = {
    -- It has hit something and bursts (BattleObject+0x0C).
    bursting: boolean,
}

local region = require("../../lib/region")

local hyper_burst = {
    state = { bursting = "bool" },
}

-- Its actions: flying, bursting; and the bursting's phases.
local FLYING, BURSTING = 0, 4
local BURST, WAIT = 0, 4
-- The bursts: region 9 on even counts and 0xB on odd ones, no spark,
-- collision types 5 (target) and 4, status 0x10 (`dword_80D9118`,
-- `dword_80D911C`, `dword_80D9150`).
local EVEN_REGION, ODD_REGION = 0x09, 0x0B
local HIT = { hit_effect = 0xFF, target = 5, self_type = 4, status = 0x10 }
-- The explosions, and how high.
local EXPLOSION, EXPLOSION_Z = 0x3D, 0xA_0000
-- Ticks between bursts, less one.
local GAP = 8

-- `sub_80D9130`: a shot on (x, y) for `owner`, with the element, Param1..4
-- `params` and the damage word `damage`. Its position is the spawner's
-- registers (the panel Y, the element and 0) until its init places it.
function hyper_burst.spawn(owner: Object, x: number, y: number, element: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("hyper-burst", Vec3.new(y, element, 0), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- Out: the region goes, hidden, then freed.
local function stop(me: Object)
    me.collision.region = 0
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80D907C`: a panel on (its X velocity is a panel step, ±1).
local function fly(me: Object)
    me.panel_x = me.panel_x + me.vel.x
    me.collision.panel_x = me.panel_x
    local cx, cy = field.center(me.panel_x, me.panel_y)
    me.pos = Vec3.new(cx, cy, me.pos.z)
    if not field.valid(me.panel_x, me.panel_y) then
        stop(me)
    end
end

-- `sub_80D90C8`: a burst; again after the gap, or done.
local function burst(me: Object)
    local r = if bit32.btest(me.timer2, 1) then ODD_REGION else EVEN_REGION
    local x, y = me.panel_x, me.panel_y
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        region = r,
        hit_effect = HIT.hit_effect,
        target = HIT.target,
        self_type = HIT.self_type,
        damage = me.damage,
        stamina = me.stamina,
        status = HIT.status,
    })
    region.effects(x, y, EXPLOSION, r, region.direction(me.alliance), EXPLOSION_Z)
    local left = me.timer2 - 1
    me.timer2 = left
    if left == 0 then
        me.collision.region = 0
        me:set_lifecycle("destroy")
        return
    end
    me.timer = GAP
    me.phase = WAIT
    me.phase_init = 0
end

-- `sub_80D9120`: the gap between bursts.
local function wait(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        me.phase = BURST
        me.phase_init = 0
    end
end

-- `sub_80D90AE`.
local function bursting(me: Object)
    local p = me.phase
    if p == BURST then
        burst(me)
    elseif p == WAIT then
        wait(me)
    else
        error(string.format("H-Burst shot phase %#x reads past its table", p))
    end
end

-- `sub_80D9024`: a first hit starts the bursts; nothing goes on once the
-- fight stops.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    local s = me.state :: State
    if c.hit_flags ~= 0 and not s.bursting then
        s.bursting = true
        me.action = BURSTING
        me.phase = 0
        me.phase_init = 0
    end
    if not battle.fighting() then
        return stop(me)
    end
    local a = me.action
    if a == FLYING then
        fly(me)
    elseif a == BURSTING then
        bursting(me)
    else
        error(string.format("H-Burst shot action %#x reads past its table", a))
    end
    c:present()
end

-- `sub_80D8FE4`.
local function init(me: Object)
    local cx, cy = field.center(me.panel_x, me.panel_y)
    me.pos = Vec3.new(cx, cy, me.pos.z)
    me.vel = Vec3.new(me.facing, me.vel.y, me.vel.z)
    me.timer2 = me:param(1) + me:param(2) * 0x100
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(0x3D, 5, 0)
    me.collision.status_base = 0x10
    me.collision:present()
    me:set_lifecycle("update")
end

-- `sub_80D8FC4` (it has no sprite).
function hyper_burst.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return hyper_burst
