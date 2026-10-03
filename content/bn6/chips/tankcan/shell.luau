--!strict
-- TankCan's shell (the original's attack object #0x3C, `sub_80CB6F8`).
-- Unseen, it steps a panel every two ticks from where it appears; its own
-- hit ends it with a fiery burst. Off the far edge it blasts the last
-- column it crossed (and cracks it), and 15, 20 and 25 ticks later
-- explosions go off over that column's three panels in an order the RNG
-- shuffles. See docs/engine/shot-chips.md §7.1.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local panels = require("@common/panels")
local regions = require("@common/regions")

local shell = {}

export type State = {
    -- The hit modifier its own hit carries (`byte_80CB6F0`'s row: 0x4B for
    -- TankCan's kind 0, 0x63 for kind 1).
    hit_mod: number,
    -- The edge blast cracks its panels (its second parameter); its damage
    -- is the shell's shifted right this far (its third).
    cracks: boolean,
    shift: number,
    -- The panels of the explosions, x | y << 4 (the game's Param2..4,
    -- shuffled), read last first.
    spots: { number },
}

-- Ticks it waits on a panel after the one it enters on (Timer2).
local WAIT = 1
local HIT_SOUND = asset.sound("bn6:burst")
local SOUND = asset.sound("bn6:cannon")
-- The edge blast: its effect (effect 0x60), its hit (what it is, region,
-- modifier), and the explosions' wait, interval and height (16.16).
local BLAST = define.effect { sprite = asset.sprite("bn6:tank-blast"), anim = 0, palette = 0 }
local BLAST_HIT_MOD = 3
local FIRST_EXPLOSION, EXPLOSION_INTERVAL, EXPLOSION_Z = 0xE, 4, 0x6_0000

-- Done: off its panels, and freed at the next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- The edge blast (`sub_80CB7BC`'s end), over (x, y): the last column the
-- shell crossed.
local function blast(me: Object, s: State, x: number, y: number)
    me.collision:set_region(nil)
    battle.shake_camera(3, 0x28)
    battle.play_sound(HIT_SOUND)
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0), BLAST, bit32.bxor(me.alliance, me.flip))
    local word = int.lsr(int.u32(me.damage + me.stamina * 0x10000), s.shift)
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = 0,
        region = regions.wide,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.thrown,
        damage = bit32.band(word, 0xFFFF),
        stamina = bit32.rshift(word, 16),
        hit_mod = BLAST_HIT_MOD,
    })
    if s.cracks then
        field.crack(x, y)
        field.crack(x, y - 1)
        field.crack(x, y + 1)
    end
    -- `sub_8000C72`: three rounds of two draws, each swapping two of the
    -- three panels.
    local here = x + y * 0x10
    local spots = { here, here - 0x10, here + 0x10 }
    panels.shuffle(spots, 3, 3)
    for i = 1, 3 do
        s.spots[i] = spots[i]
    end
    me.timer2 = 3
    me.timer = FIRST_EXPLOSION
    me.action = 4
    me.phase, me.phase_init = 0, 0
end

-- `sub_80CB7BC`: wait out the panel, then step forward; off the far edge,
-- the blast.
local function fly(me: Object, s: State)
    if me.phase == 0 and me.phase_init == 0 then
        me.phase = 1
        me.timer = me.timer2
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.phase = 0
    me.panel_x = me.panel_x + me.facing
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    if field.valid(me.panel_x, me.panel_y) then
        return
    end
    blast(me, s, me.panel_x - me.facing, me.panel_y)
end

-- `sub_80CB898`: every 5 ticks after the first 15, an explosion over the
-- next panel (none for one off the field's rows); after the three (and a
-- fourth turn that reads the shell's kind, which is no panel), it ends.
local function explode(me: Object, s: State)
    if me.timer ~= 0 then
        me.timer -= 1
        return
    end
    me.timer = EXPLOSION_INTERVAL
    local n = me.timer2
    local spot = if n >= 1 then s.spots[n] else 0
    local y, x = bit32.rshift(spot, 4), bit32.band(spot, 7)
    if y ~= 0 and y <= 3 then
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, EXPLOSION_Z), effects.explosion)
    end
    n -= 1
    me.timer2 = n
    if n < 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80CB762`: while it has a region, a hit ends it with a burst and
-- the battle's end ends it; then its action.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.region ~= nil then
        if c.hit_flags ~= 0 then
            battle.play_sound(HIT_SOUND)
            battle.effect(me.pos, effects.shell_burst)
            finish(me)
            return
        elseif battle.over() then
            finish(me)
            return
        end
    end
    if me.action == 0 then
        fly(me, s)
    else
        explode(me, s)
    end
    c:present()
end

-- `sub_80CB71C`: on its panel, an attack of its side with its hit
-- modifier and no spark; its first update; the shot's sound.
local function init(me: Object)
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, (me.state :: State).hit_mod)
    me.collision:set_hit_effect(nil)
    me.collision:present()
    me.timer2 = WAIT
    me:set_lifecycle("update")
    tick(me)
    battle.play_sound(SOUND)
end

shell.kind = define.kind {
    id = "bn6:tankcan/shell",
    pool = "attack",
    state = { hit_mod = "u8", cracks = "bool", shift = "u8", spots = "u8[3]" },
    -- `sub_80CB6F8`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- `sub_80CB8DE`: a shell from `owner` on panel (x, y) at height `z`
-- (16.16), element 0, with the damage word; nil if the attack pool is
-- full. It takes the owner's side and flip, runs later this tick, right
-- after its spawner, and keeps its Z.
function shell.fire(owner: Object, x: number, y: number, z: number, damage: number, spec: ShellSpec): Object?
    local o = battle.spawn(shell.kind, Vec3.new(0, 0, z))
    if o then
        local s = o.state :: State
        s.hit_mod = spec.hit_mod
        s.cracks = spec.cracks
        s.shift = spec.shift
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

return shell
