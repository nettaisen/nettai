--!strict
-- BlueMoon's ray (EXE4's attack object #0x87, 0x080DE160; EXE4's own, though its first action is EXE6's SunMoon
-- moonlight's, `sub_80DC180`): a shaft of light on its moon's panel. After 3 ticks it hits the panel three columns
-- toward the enemy once with the chip's damage (a thrown piercing break), then on each of the next nine ticks with a
-- hit of no damage that strips a power from the navi it reaches: each carries a NaviCust bug code
-- (../../rules/navicust/bugs): SuperArmor, FloatShoes, AirShoes and UnderShirt off, Attack, Rapid and Charge to 0, the
-- charged shot back to the navi's own and no B+Left (0x080DE284's words). It goes 91 ticks after its first hit.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")

local ray = {}

local SPRITE = asset.sprite("blue-moon-ray")
local SOUND = asset.sound("blue-moon-ray")
-- Its hits: one panel three columns ahead (the region word 0x1705FF01: no spark, a thrown piercing break hitting
-- navis); the damage's hit modifier 3, the strips' 1.
local AHEAD = 3
local HIT_MOD, STRIP_HIT_MOD = 3, 1
-- The strips' element (their spawner's register 5).
local STRIP_ELEMENT = 5
-- The strips, one a tick (0x080DE284: each word's bug code and its argument).
local STRIPS = {
    { bug = 0x01, arg = 0 },
    { bug = 0x02, arg = 0 },
    { bug = 0x03, arg = 0 },
    { bug = 0x04, arg = 0 },
    { bug = 0x05, arg = 0 },
    { bug = 0x06, arg = 0 },
    { bug = 0x07, arg = 0 },
    { bug = 0x0A, arg = 1 },
    { bug = 0x0C, arg = 0xFF },
}
-- How long it lasts from its first hit.
local LASTS = 90

-- A hit on the panel three columns ahead that resolves while dimmed (`sub_80C53A6`, 0x080CD81E).
local function hit(me: Object, element: number, damage: number, hit_mod: number, bug: number?, arg: number?)
    local box = battle.hitbox(me, {
        panel_x = me.panel_x + me.facing * AHEAD,
        panel_y = me.panel_y,
        element = element,
        region = regions.single,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.thrown_piercing_break,
        damage = bit32.band(damage, 0xFFFF),
        stamina = bit32.rshift(damage, 16),
        hit_mod = hit_mod,
        bug = bug,
        bug_arg = arg,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- Action 0 (0x080DE1F0, EXE6's `sub_80DC180`): 3 ticks.
local function wait(me: Object)
    if me.phase_init == 0 then
        me.timer = 3
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t <= 0 then
        me:set_action(4)
    end
end

-- Action 4 (0x080DE212): its hit, then a strip a tick; 91 ticks from its first.
local function shine(me: Object)
    if me.phase_init == 0 then
        me.timer = LASTS
        me.timer2 = 0
        hit(me, me.element, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), HIT_MOD, nil, nil)
        me.phase_init = 4
    end
    local n = me.timer2
    if n < #STRIPS then
        local strip = STRIPS[n + 1]
        hit(me, STRIP_ELEMENT, 0, STRIP_HIT_MOD, strip.bug, strip.arg)
        me.timer2 = n + 1
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t < 0 then
        me:set_lifecycle("destroy")
    end
end

-- 0x080DE1D0 (EXE6's `sub_80DC160`): by action, then its sprite steps.
local function tick(me: Object)
    local a = me.action
    if a == 0 then
        wait(me)
    elseif a == 4 then
        shine(me)
    else
        error(string.format("BlueMoon's ray: action %#x reads past its table (0x080DE1E8)", a))
    end
    me:update_sprite()
end

-- 0x080DE180: its sprite (no shadow) in animation 0, stepped once, turned to its side's way, on its panel's ground;
-- its sound.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    sprite.shadow = "with_sprite"
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

local kind = new.kind {
    id = "bluemoon/ray",
    pool = "attack",
    -- 0x080DE160.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- 0x080DE2A8: a ray on (x, y) with `element` and the damage word, of `owner`'s side alone; it acts while dimmed.
-- (Its position is the spawner's registers, which its init overwrites.)
function ray.spawn(owner: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance = owner.alliance
        o.run_while_dimmed = true
    end
    return o
end

return ray
