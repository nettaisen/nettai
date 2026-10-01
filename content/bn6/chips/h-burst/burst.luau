--!strict
-- H-Burst's shot (attack object #0x9E, `sub_80D8FC4`): an unseen probe
-- stepping a panel a tick along its row (status 0x10) until it hits
-- something. There it bursts again and again, every 9 ticks: a hit on the
-- panel over the plus around it and the X around it in turn (status
-- 0x10), with an explosion 10 pixels up on each of their panels. It ends
-- when it leaves the field, when the bursts are done, or when the fight
-- stops.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")

local burst = {}

export type State = {
    -- It has hit something and bursts (BattleObject+0x0C).
    bursting: boolean,
    -- The bursts it makes (the original's first two parameters, a
    -- halfword).
    bursts: number,
}

-- Its actions: flying, bursting; and the bursting's phases.
local FLYING, BURSTING = 0, 4
local BURST, WAIT = 0, 4
-- What it hits with: a status 0x10 hit, no spark (`dword_80D9118`,
-- `dword_80D911C`, `dword_80D9150`), on the plus around its panel on even
-- counts and the X on odd ones.
local STATUS = 0x10
local HIT = { is = collision.attack, hits = collision.hits_navis }
-- The explosions (effect 0x3D), and how high.
local EXPLOSION = define.effect { sprite = asset.sprite("reflected-shot"), anim = 0, palette = 2 }
local EXPLOSION_Z = 0xA_0000
-- Ticks between bursts, less one.
local GAP = 8

-- Out: the region goes, hidden, then freed.
local function stop(me: Object)
    me.collision:set_region(nil)
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
local function blast(me: Object)
    local r = if bit32.btest(me.timer2, 1) then regions.x else regions.plus
    local x, y = me.panel_x, me.panel_y
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        region = r,
        -- None.
        hit_effect = 0xFF,
        target = HIT.hits,
        self_type = HIT.is,
        damage = me.damage,
        stamina = me.stamina,
        status = STATUS,
    })
    battle.region_effects(x, y, r, me.alliance, EXPLOSION, EXPLOSION_Z)
    local left = me.timer2 - 1
    me.timer2 = left
    if left == 0 then
        me.collision:set_region(nil)
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
        blast(me)
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
    me.timer2 = (me.state :: State).bursts
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.probe, collision.hits_navis, 0)
    me.collision.status_base = STATUS
    me.collision:present()
    me:set_lifecycle("update")
end

burst.kind = define.kind {
    id = "h-burst/burst",
    pool = "attack",
    state = { bursting = "bool", bursts = "u16" },
    -- `sub_80D8FC4` (it has no sprite).
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

-- `sub_80D9130`: a shot on (x, y) for `owner` that bursts `bursts` times,
-- with the element and the damage word `damage`. Its position is the
-- spawner's registers (the panel Y, the element and 0) until its init
-- places it.
function burst.spawn(owner: Object, x: number, y: number, element: number, bursts: number, damage: number): Object?
    local o = battle.spawn(burst.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        local s = o.state :: State
        s.bursts = bursts
    end
    return o
end

return burst
