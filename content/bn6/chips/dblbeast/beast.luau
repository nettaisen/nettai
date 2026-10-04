--!strict
-- What DblBeast's two beasts share (docs/engine/dimming-chip-effects.md
-- §19): Gregar's beast (./gregar, actor #0x33) and Falzar's (./falzar,
-- actor #0x34) list the enemy's bodies, then attack them one after another,
-- each attack from a panel a pattern of offsets around the next target
-- gives; between attacks they wait 31 ticks. Their hits add their own
-- damage to the controller's bonus.

local panels = require("@exelib/panels")
local collision = require("../../rules/collision")

local beast = {}

-- One state for both kinds.
beast.STATE = {
    controller = "object",
    base = "u32",
    next = "u8",
    count = "u8",
    missed = "bool",
    targets = "u8[18]",
    held = "object",
    held2 = "object",
}

-- The ticks of an attack and of a wait.
beast.TICKS = 0x1E

-- The enemy's bodies, by the beast's side (`off_80C1EEC`).
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }
-- A panel to attack from: on the field, nothing on it or reserved.
local FREE_REQUIRE, FREE_FORBID = 0x00010000, 0x0F880080
-- At most this many of a pattern's entries are tried.
local MAX_TRIED = 6

-- `sub_80C1ECE` / `sub_80C24F4`: the panels with an enemy body on them
-- but its own (rows 3 to 1, columns 6 to 1); the first target first.
function beast.list_targets(me: Object)
    local s = me.state :: DblBeastState
    local list = panels.filtered_except(me, ENEMY_BODY[me.alliance], 0)
    for i, p in list do
        s.targets[i] = p
    end
    s.count = #list
    s.next = 0
end

-- `sub_80C1EF4` / `sub_80C2518`: from the next target on (to the last, not
-- around), the first free panel of the pattern around it; the next target
-- then follows it (back to the first after the last). A candidate on the
-- user's panel is remembered: with nothing free, the user's panel (facing
-- the enemy) if one was. Else nil.
function beast.attack_panel(me: Object, pattern: DblBeastPattern): (number?, number, number)
    local s = me.state :: DblBeastState
    local user = me.related1 :: Object
    -- object_getEnemyDirection: the side's forward.
    local dir = 1 - 2 * me.alliance
    local on_user = false
    local i = s.next
    repeat
        local t = s.targets[i + 1]
        local tx, ty = panels.x(t), panels.y(t)
        for n = 1, math.min(#pattern, MAX_TRIED) do
            local e = pattern[n]
            local x, y = tx + e[2] * dir, ty + e[3]
            if field.check(x, y, FREE_REQUIRE, FREE_FORBID) then
                i += 1
                s.next = if i >= s.count then 0 else i
                return x, y, e[1]
            end
            if x == user.panel_x and y == user.panel_y then
                on_user = true
            end
        end
        i += 1
    until i >= s.count
    if on_user then
        s.next = 0
        return user.panel_x, user.panel_y, 0
    end
    return nil, 0, 0
end

-- An attack's start: the last attack's panel let go (all but the first
-- attack), then the pattern's panel taken (facing as it says, reserved);
-- without one, the attack is a miss.
function beast.take_panel(me: Object, pattern: DblBeastPattern, drop_last: boolean): boolean
    if drop_last then
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    end
    local x, y, flip = beast.attack_panel(me, pattern)
    if not x then
        (me.state :: DblBeastState).missed = true
        return false
    end
    me.flip = flip
    me.panel_x, me.panel_y = x, y
    me:set_coordinates_from_panel()
    me.sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.future_panel_x, me.future_panel_y = x, y
    me:reserve_panel(x, y)
    return true
end

-- `sub_80C205C` / `sub_80C2678`: back to the rest spot, on the user's
-- panel 10 pixels to one side (`ahead`: forward, else back), on the ground.
function beast.rest(me: Object, ahead: boolean)
    local user = me.related1 :: Object
    me.panel_x, me.panel_y = user.panel_x, user.panel_y
    me:set_coordinates_from_panel()
    local dx = me.facing * 0xA_0000
    local p = me.pos
    me.pos = Vec3.new(if ahead then p.x + dx else p.x - dx, p.y, 0)
end

-- The waits between attacks: 31 ticks (the first sets the count), then
-- `next`.
function beast.wait(me: Object, next: number)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = beast.TICKS
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me.action, me.phase, me.phase_init = next, 0, 0
    end
end

-- The appearing (actions 0): shown, with a sound, 3 ticks.
local SOUND_APPEAR = asset.sound("bn6:appear")
local ANIM_APPEAR, APPEAR_TICKS = 3, 3
function beast.appear(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = ANIM_APPEAR
        me.visible = true
        battle.play_sound(SOUND_APPEAR)
        me.timer = APPEAR_TICKS
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me.action, me.phase, me.phase_init = 4, 0, 0
    end
end

-- A hit (`sub_80C2000`, `sub_80C2600`): a one-tick hit on the panel `ahead`
-- panels in front, of its element, turned the way the beast faces, its
-- damage on top of the base word, with hit modifier 3, resolving while
-- dimmed.
local HIT_MOD = 3
function beast.hit(me: Object, h: DblBeastHit, x: number?)
    local s = me.state :: DblBeastState
    local word = int.u32(h.damage + s.base)
    local o = battle.hitbox(me, {
        panel_x = x or me.panel_x + me.facing * h.ahead,
        panel_y = me.panel_y,
        element = me.element,
        z = h.z or 0,
        region = h.region,
        hit_effect = h.spark,
        target = collision.hits_navis,
        self_type = h.is,
        damage = bit32.band(word, 0xFFFF),
        stamina = bit32.rshift(word, 16),
        hit_mod = HIT_MOD,
    })
    if o then
        o.run_while_dimmed = true
    end
end

-- The beast is done (`sub_80C1E7A`'s and `sub_80C2498`'s end): its flag
-- cleared (the original writes it wherever the controller was), and
-- destroyed (freed at its next update).
function beast.leave(me: Object, flag: "gregar_out" | "falzar_out")
    local c = (me.state :: DblBeastState).controller
    if c and c.active then
        (c.state :: { [string]: boolean })[flag] = false
    end
    me:set_lifecycle("destroy")
end

-- The beasts' common init (`sub_80C1A34`, `sub_80C215C`): at rest, drawn
-- with `sprite` (no shadow, animation 0, facing its way), with its
-- identity's NameID (its attachments sit at that sprite's attach
-- points); no attack has missed.
function beast.init(me: Object, sprite: SpriteAsset, ahead: boolean, identity: Identity)
    beast.rest(me, ahead)
    local sp = me.sprite
    sp:load(sprite)
    sp.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0
    sp:set_animation(0)
    sp.palette = 0
    sp:set_flip(bit32.bxor(me.alliance, me.flip))
    me.identity = identity
    local s = me.state :: DblBeastState
    s.missed = false
end

-- `sub_80C1EAC` / `sub_80C24D2`: a beast of `kind` for `user`, of its
-- side and facing, with `controller` waiting on it (the flag `flag` set)
-- and the damage word `base`. (Spawned at the registers; its init places
-- it.)
function beast.spawn(kind: Kind, controller: Object, user: Object, element: number, base: number, flag: "gregar_out" | "falzar_out"): Object?
    local o = battle.spawn(kind, Vec3.zero)
    if not o then
        return nil
    end
    o.element = element
    local s = o.state :: DblBeastState
    s.base = base
    s.controller = controller
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    ;(controller.state :: { [string]: boolean })[flag] = true
    return o
end

return beast
