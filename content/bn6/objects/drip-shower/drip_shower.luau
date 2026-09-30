--!strict
-- SpoutMan's water spray (attack object #0x29, `sub_80C91A0`): DripShwr's.
-- It stands on its owner's panel and sweeps a pair of the panels around
-- it, a quarter turn every Param3 ticks, with a hit region on each (lasting
-- Param3 ticks) and the next pair highlighted. Param2 turns in all. Standing
-- still while dimmed; gone when the battle is over, or (unless Param1 is 4)
-- once its owner is out of action 0xC. [unverified: no lab scenario reaches
-- it; docs/engine/standard-chips.md]

export type State = {
    -- The quarter it sprays (ExtraVars+0).
    quarter: number,
    -- Ticks to its next splash sound (ExtraVars+4).
    sound: number,
}

local drip_shower = {
    state = { quarter = "u8", sound = "i32" },
}

local SPRITE = "10-22"
-- Its hit regions: region 1, hit spark 2, target 5, self type 4 (Param
-- word 0x4050201), hit modifier 3.
local REGION, SPARK, TARGET, SELF, HIT_MOD = 1, 2, 5, 4, 3
-- The two panels of each quarter (`byte_80C93B0`), and which quarter comes
-- in turn by side (`byte_80C93C4`); the animation by side and turn
-- (`byte_80C92C4`).
local QUARTERS = {
    [0] = { { -1, 0 }, { 1, 0 } },
    { { -1, -1 }, { 1, 1 } },
    { { 0, -1 }, { 0, 1 } },
    { { 1, -1 }, { -1, 1 } },
}
local ORDER = { [0] = { [0] = 0, 3, 2, 1 }, [1] = { [0] = 0, 1, 2, 3 } }
local ANIMS = { [0] = { [0] = 0, 1, 2, 3 }, [1] = { [0] = 0, 3, 2, 1 } }

-- `sub_80C92CC`: the spray for `owner` on (x, y), with the damage word
-- `damage` and parameters { kind (4: the link chip's), turns, ticks a
-- quarter }. It runs while dimmed (and stands still); its position is the
-- registers (the panel Y, the element and `z`) until its init.
function drip_shower.spawn(owner: Object, x: number, y: number, element: number, params: { number }, damage: number, z: number): Object?
    local o = battle.spawn_kind("drip-shower", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

local function quarter_panels(me: Object, quarter: number): { { number } }
    local q = QUARTERS[ORDER[me.alliance][quarter]]
    if q == nil then
        error(string.format("drip shower quarter %d reads past byte_80C93C4", quarter))
    end
    return q
end

-- `sub_80C9360`: highlight the quarter's panels.
local function highlight(me: Object, quarter: number)
    for _, p in quarter_panels(me, quarter) do
        local x, y = me.panel_x + p[1], me.panel_y + p[2]
        if field.valid(x, y) then
            field.highlight(x, y)
        end
    end
end

-- `sub_80C92F4`: highlight the quarter's panels and hit them for `ticks`.
local function splash(me: Object, quarter: number, ticks: number)
    for _, p in quarter_panels(me, quarter) do
        local x, y = me.panel_x + p[1], me.panel_y + p[2]
        if field.valid(x, y) then
            field.highlight(x, y)
            local hit = battle.hitbox(me, {
                panel_x = x,
                panel_y = y,
                element = me.element,
                region = REGION,
                hit_effect = SPARK,
                target = TARGET,
                self_type = SELF,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = HIT_MOD,
            })
            if hit then
                hit.timer = ticks
            end
        end
    end
end

-- `sub_80C9238`: sweep, a quarter every Param3 ticks, for Param2 turns.
local function sweep(me: Object)
    local s = me.state :: State
    local ticks = me:param(3)
    if me.phase == 0 then
        me.phase = 4
        s.quarter = 0
        me.timer = ticks * 4 * me:param(2) + ticks
        me.timer2 = ticks
        splash(me, 0, ticks)
        s.sound = 0
    end
    local sound = s.sound - 1
    s.sound = sound
    if sound <= 0 then
        battle.play_sound(0x128)
        s.sound = 0x10
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_lifecycle("destroy")
        return
    end
    highlight(me, s.quarter)
    local left = me.timer2 - 1
    me.timer2 = left
    if left > 0 then
        return
    end
    local q = (s.quarter + 1) % 4
    s.quarter = q
    me.anim = ANIMS[me.alliance][q]
    me.timer2 = ticks
    splash(me, q, ticks)
end

-- `sub_80C91FE`.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    if me:param(1) ~= 4 and (me.related1 :: Object).action ~= 0xC then
        me:set_lifecycle("destroy")
        return
    end
    if battle.dimmed() then
        return
    end
    if me.action ~= 0 then
        error(string.format("drip shower action %#x reads past off_80C9234", me.action))
    end
    sweep(me)
end

-- `sub_80C91C4`: on its panel, on the ground.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    me:set_lifecycle("update")
end

-- `sub_80C91A0`.
function drip_shower.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
        return
    end
    me:update_sprite_while_paused()
end

return drip_shower
