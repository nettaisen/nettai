--!strict
-- TankCan's shell (EXE6's attack object #0x3C, `sub_80CB6F8`; EXE5's the same number and code, 0x080CC9EC, but for its
-- look, its edge blast's collision type and an unshifted blast). Unseen, it steps a panel every two ticks from where
-- it appears; its own hit ends it with a fiery burst. Off the far edge it blasts the last column it crossed (and
-- cracks it), and 15, 20 and 25 ticks later explosions go off over that column's three panels in an order the RNG
-- shuffles.
--
-- Each game makes its kind with `shell.make_kind(id, look)` (EXE6's chips/tankcan/shell, EXE5's chips/tankcan1).

local panels = require("../panels")
local regions = require("../regions")

local shell = {}

-- A shell's own hit and its blast (`ShellSpec`): its hit modifier, whether the blast cracks its panels, and the
-- blast's damage shift (EXE5's: none).
export type Spec = ShellSpec

-- A game's shell: what it is and hits, its sounds (the shot, a hit or blast), the edge blast's look and collision
-- type, the explosions' and a hit's burst's effects.
export type Look = TankShellLook

export type State = {
    -- The hit modifier its own hit carries (a row of `byte_80CB6F0`: 0x4B for TankCan's kind 0, 0x63 for kind 1).
    hit_mod: number,
    -- The edge blast cracks its panels (its second parameter); its damage is the shell's shifted right this far
    -- (its third).
    cracks: boolean,
    shift: number,
    -- The panels of the explosions, x | y << 4 (the game's Param2..4, shuffled), read last first.
    spots: { number },
}

-- Ticks it waits on a panel after the one it enters on (Timer2).
local WAIT = 1
-- The edge blast's hit modifier, and the explosions' wait, interval and height (16.16).
local BLAST_HIT_MOD = 3
local FIRST_EXPLOSION, EXPLOSION_INTERVAL, EXPLOSION_Z = 0xE, 4, 0x6_0000

-- Done: off its panels, and freed at the next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- The edge blast (`sub_80CB7BC`'s end), over (x, y): the last column the shell crossed.
local function blast(me: Object, look: Look, s: State, x: number, y: number)
    me.collision:set_region(nil)
    battle.shake_camera(3, 0x28)
    battle.play_sound(look.hit_sound)
    local cx, cy = field.center(x, y)
    battle.effect(Vec3.new(cx, cy, 0), look.blast, bit32.bxor(me.alliance, me.flip))
    local word = int.lsr(int.u32(me.damage + me.stamina * 0x10000), s.shift)
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = 0,
        region = regions.wide,
        hit_effect = nil,
        target = look.hits,
        self_type = look.blast_is,
        damage = bit32.band(word, 0xFFFF),
        stamina = bit32.rshift(word, 16),
        hit_mod = BLAST_HIT_MOD,
    })
    if s.cracks then
        field.crack(x, y)
        field.crack(x, y - 1)
        field.crack(x, y + 1)
    end
    -- `sub_8000C72`: three rounds of two draws, each swapping two of the three panels.
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

-- `sub_80CB7BC`: wait out the panel, then step forward; off the far edge, the blast.
local function fly(me: Object, look: Look, s: State)
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
    blast(me, look, s, me.panel_x - me.facing, me.panel_y)
end

-- `sub_80CB898`: every 5 ticks after the first 15, an explosion over the next panel (none for one off the field's
-- rows); after the three (and a fourth turn that reads the shell's kind, which is no panel), it ends.
local function explode(me: Object, look: Look, s: State)
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
        battle.effect(Vec3.new(cx, cy, EXPLOSION_Z), look.explosion)
    end
    n -= 1
    me.timer2 = n
    if n < 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80CB762`: while it has a region, a hit ends it with a burst and the battle's end ends it; then its action.
local function tick(me: Object, look: Look)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.region ~= nil then
        if c.hit_flags ~= 0 then
            battle.play_sound(look.hit_sound)
            battle.effect(me.pos, look.burst)
            finish(me)
            return
        elseif battle.over() then
            finish(me)
            return
        end
    end
    if me.action == 0 then
        fly(me, look, s)
    else
        explode(me, look, s)
    end
    c:present()
end

-- `sub_80CB71C`: on its panel, an attack of its side with its hit modifier and no spark; its first update; the
-- shot's sound. (EXE5's also spawns effect object 0x83 there with battle flag 0x40, 0x080CCA5C: never in a
-- netbattle, and not ported.)
local function init(me: Object, look: Look)
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(look.is, look.hits, (me.state :: State).hit_mod)
    me.collision:set_hit_effect(nil)
    me.collision:present()
    me.timer2 = WAIT
    me:set_lifecycle("update")
    tick(me, look)
    battle.play_sound(look.sound)
end

-- A game's shell kind `id`, of `look`.
function shell.make_kind(id: string, look: Look): Kind
    return new.kind {
        id = id,
        pool = "attack",
        state = { hit_mod = "u8", cracks = "bool", shift = "u8", spots = "u8[3]" },
        -- `sub_80CB6F8`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
                if not me.active then
                    return
                end
            elseif l == "update" then
                tick(me, look)
            else
                me:destroy()
                return
            end
            me:update_sprite()
        end,
    }
end

-- `sub_80CB8DE`: a shell of `kind` from `owner` on panel (x, y) at height `z` (16.16), element 0, with the damage
-- word; nil if the attack pool is full. It takes the owner's side and flip, runs later this tick, right after its
-- spawner, and keeps its Z.
function shell.fire(kind: Kind, owner: Object, x: number, y: number, z: number, damage: number, spec: Spec): Object?
    local o = battle.spawn(kind, Vec3.new(0, 0, z))
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
