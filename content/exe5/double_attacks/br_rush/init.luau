--!strict
-- BR-RUSH (the DS's chip 0x166; its Japanese name Brain Rush), the Double Attack of MegaMan and Colonel
-- (docs/design/exe5ds-map.md §5, §5.2: its controller the DS's effect object 0x9D, its navis its T1 0x3E). Colonel
-- looks down his row for the first enemy body whose panel before it is solid and free (else both leave), throws his
-- cape from that panel, and both leap off the field; once the cape stops on what it caught they drop onto that panel,
-- MegaMan firing three shots and Colonel striking three times over 40 ticks, then a sword's finisher each: eight hits
-- on the enemy's panel. The chip's record and name are the DS's (written by tools/exe5/gen_content.py,
-- bn6battle-verify, from the DS ROMs); its record's use is the cannon's (action 0x14, subtype 0), which nothing starts:
-- no folder or hand holds it. Sound 0x2F, which the DS plays with 0xFB as the strikes start, is the DS's own: left out
-- (the user's decision of 2026-10-20).

local attachment = require("../objects/attachment")
local buster = require("../lib/buster")
local cannon_chips = require("../chips/cannon")
local colonel = require("../navis/colonel").colonel
local collision = require("../rules/collision")
local double_attack = require("../lib/double_attack")
local megaman = require("../navis/megaman").megaman
local regions = require("@exelib/regions")

-- The navis' own beside the shared (`DoubleAttackNavi`): Colonel's cape while he holds it (+0x74).
type State = DoubleAttackNavi & { cape: Object? }

-- The cape's (the GBA's attack object 2, Colonel's cape, 0x080C4AE4; the DS's 0x02203190 with its Param2 variants):
-- whom it reports to (its +0x60, a pointer to its thrower's +0x74), and whether its thrower let it go (+0x64).
type CapeState = { owner: Object?, released: boolean }

local CHARGE_TICKS = 0x3C
local LOOK_TICKS, WIND_TICKS, LEAP_TICKS, DROP_TICKS, READY_TICKS = 0x1E, 0xF, 4, 4, 0xF
local RUSH_TICKS, FINISH_WAIT, FINISH_TICKS, AFTER_FINISH, OUT_TICKS, AFTER_TICKS = 0x28, 0x1E, 0x15, 0x14, 4, 0x1E
-- The rush's hits (each navi's three, at these ticks left of its 40), MegaMan's shots (his first as it starts, then
-- these), the finisher's hit; Colonel's cape leaves this many ticks into his throw.
local HITS_AT: { number } = { 0x23, 0x1E, 0xA }
local SHOTS_AT: { number } = { 0x1E, 0xF }
local FINISHER_AT, CAPE_AT = 0xB, 0xB
-- Where both wait off the field: panel (0, 0), 190 pixels up.
local AWAY_Z = 0xBE0000
local HIT_MOD = 3

local ANIM_IDLE, ANIM_LAND, ANIM_LEAP, ANIM_SHOOT, ANIM_SWING = 0, 3, 4, 0xE, 5
local ANIM_THROW, ANIM_COLONEL_LEAP, ANIM_COLONEL_LAND, ANIM_RUSH, ANIM_FINISH = 0x11, 0x17, 0x16, 0x13, 0x10
local SOUND_SHOT = asset.sound("buster-shot")
local SOUND_SWING = asset.sound("sword-swing")
local SOUND_THROW = asset.sound("cape-throw")
local SOUND_RUSH = asset.sound("tenguman-nose")
local SOUND_FINISH = asset.sound("big-sword-swing")
-- Their swords (the attachment table's row 3): MegaMan's in its animation 0, Colonel's in 7.
local SWORD = attachment.look { sprite = asset.sprite("sword"), palette = 0 }
local MEGAMAN_SWORD_ANIM, COLONEL_SWORD_ANIM = 0, 7
-- What Colonel looks for (0x020DDE30, by his side): an enemy body; and the panel before it (0x020DDE38): solid, with no
-- object, reservation or body of his side's on it.
local ENEMY_BODY = { [0] = 0x04000000, [1] = 0x08000000 }
local STANDABLE = 0x10
local TAKEN = { [0] = 0x07800000, [1] = 0x0B800000 }

-- The cape: its sprite and animations (`sub_8011504`'s 0x0019101E: 10-1E, the pack's `colonel-cape`, animation 25,
-- no shadow; caught, 26); 20 pixels up as it flies, 4 pixels a tick (its Param1, 0x40, by 0x1000), 10 ticks before
-- it goes; on screen while its X is within 160 pixels of the middle.
local CAPE_SPRITE = asset.sprite("colonel-cape")
local CAPE_ANIM_FLYING, CAPE_ANIM_CAUGHT = 0x19, 0x1A
local CAPE_Z = 0x140000
local CAPE_SPEED = 0x40 * 0x1000
local CAPE_WAIT = 0xA
local SCREEN_HALF = 0xA0
-- Its actions.
local CAPE_WAITS, CAPE_FLIES, CAPE_CAUGHT, CAPE_LET_GO, CAPE_SLIDES = 0x0, 0x4, 0x8, 0xC, 0x10

-- The cape's thrower's hold of it ends (the DS writes 0 through the cape's +0x60).
local function let_go_of(me: Object)
    local owner = (me.state :: CapeState).owner
    if owner then
        (owner.state :: State).cape = nil
    end
end

-- The cape goes: its region off, its thrower's hold ended, destroyed.
local function cape_gone(me: Object)
    me.collision:set_region(nil)
    let_go_of(me)
    me:set_lifecycle("destroy")
end

-- 0x02203268: 10 ticks; then it flies (the DS's: to the slide with its Param2 1, but its test reads the action it is
-- about to set, so it always flies).
local function cape_waits(me: Object)
    if me.phase_init == 0 then
        me.timer = CAPE_WAIT
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(CAPE_FLIES)
    end
end

-- 0x02203344: its X on screen.
local function on_screen(me: Object): boolean
    local x = me.pos.x // 0x10000 + SCREEN_HALF
    return x >= 0 and x <= 2 * SCREEN_HALF
end

-- 0x02203294: forward, its panels after it; off screen it goes, having caught something it stops, else it lights
-- its panel.
local function cape_flies(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not on_screen(me) then
        cape_gone(me)
        return
    end
    if me.collision.hit_flags ~= 0 then
        me.collision:set_region(nil)
        me:set_action(CAPE_CAUGHT)
        return
    end
    field.highlight(me.panel_x, me.panel_y)
end

-- 0x022032E4: caught: on the ground on its panel, turned around; while its thrower holds it, it waits to be let go,
-- else it goes.
local function cape_caught(me: Object)
    if me.phase_init == 0 then
        me.anim = CAPE_ANIM_CAUGHT
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, 0)
        me.sprite:set_flip(bit32.bxor(bit32.bxor(me.alliance, me.flip), 1))
        me.phase_init = 4
    end
    local s = me.state :: CapeState
    local owner = s.owner
    if owner and (owner.state :: State).cape ~= nil then
        if s.released then
            me:set_action(CAPE_LET_GO)
        end
        return
    end
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x0220332C: let go, it waits for its thrower's hold to end, then goes.
local function cape_let_go(me: Object)
    local owner = (me.state :: CapeState).owner
    if owner and (owner.state :: State).cape ~= nil then
        return
    end
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x02203358 (the slide its wait's test means for its Param2 1, which the test never takes): forward 10 ticks, then
-- caught.
local function cape_slides(me: Object)
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    if me.phase_init == 0 then
        me.timer = CAPE_WAIT
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me.collision:set_region(nil)
        me:set_action(CAPE_CAUGHT)
    end
end

-- 0x02203234: the hits against it resolve and spark; when the battle is over it goes; else its action; its region
-- presented again.
local function cape_tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        let_go_of(me)
        me:set_lifecycle("destroy")
    else
        local a = me.action
        if a == CAPE_WAITS then
            cape_waits(me)
        elseif a == CAPE_FLIES then
            cape_flies(me)
        elseif a == CAPE_CAUGHT then
            cape_caught(me)
        elseif a == CAPE_LET_GO then
            cape_let_go(me)
        elseif a == CAPE_SLIDES then
            cape_slides(me)
        else
            error(string.format("Colonel's cape: action %#x reads past its table (0x022278FC)", a))
        end
    end
    c:present()
end

-- 0x022031B0 (the GBA's 0x080C4B0C with the DS's Param2 2: no immobilizing status, its own palette, its region kept):
-- its sprite and an attack against navis with no hit modifier and no spark, on its panel 20 pixels up, flying forward,
-- shown; then its first update.
local function cape_init(me: Object)
    local sprite = me.sprite
    sprite:load(CAPE_SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = CAPE_ANIM_FLYING, 0xFF
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, 0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.collision:set_hit_effect(nil)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, CAPE_Z)
    me.vel = Vec3.new(me.facing * CAPE_SPEED, me.vel.y, me.vel.z)
    me.visible = true
    (me.state :: CapeState).released = false
    me:set_lifecycle("update")
    cape_tick(me)
end

local cape = new.kind {
    id = "br-rush/cape",
    pool = "attack",
    state = { owner = "object", released = "bool" },
    -- 0x02203190: by state; then its sprite steps, paused or not (its Param2 2).
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            cape_init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            cape_tick(me)
        else
            me:destroy()
            return
        end
        me:step_sprite()
    end,
}

-- `exe5_80C4C98`: Colonel's cape on (x, y), his, with no element and no damage, running while dimmed; he holds it.
local function throw_cape(me: Object, x: number, y: number)
    local o = battle.spawn(cape, Vec3.new(y, 0, 0))
    if not o then
        return
    end
    o.panel_x, o.panel_y = x, y
    o.element = 0
    o:set_damage_word(0)
    o.alliance, o.flip = me.alliance, me.flip
    o.related1 = me
    o.run_while_dimmed = true
    (me.state :: State).cape = o
    ;(o.state :: CapeState).owner = me
end

-- `exe5_80C4CB8`: the cape caught something and stopped (its action 8, on a panel).
local function cape_stopped(o: Object): boolean
    return o.action == CAPE_CAUGHT and o.panel_x ~= 0
end

-- A hit on the panel ahead (the enemy's): no spark, as a slash, hit modifier 3 (0x02033360, 0x02033720).
local function hit(me: Object)
    double_attack.hit(me, {
        panel_x = me.panel_x + me.facing,
        panel_y = me.panel_y,
        element = 0,
        z = 0,
        region = regions.single,
        target = collision.hits_navis,
        self_type = collision.slash,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
end

local function appear(me: Object)
    double_attack.appear(me, 4, 0x10)
end

-- 0x020333E8: Colonel appears (the shared step) and puts on what his identity wears (his cape, `sub_8011420` with
-- Param3 1, a player's identity's: `sub_800F29C`'s actor type 2).
local function colonel_appear(me: Object)
    double_attack.appear(me, 4, 0x10, true)
end

-- 0x02033040: MegaMan's charge; then his future panel the one in front (Colonel's search overwrites it).
local function megaman_charge(me: Object)
    if not double_attack.charge(me, CHARGE_TICKS) then
        return
    end
    me.future_panel_x, me.future_panel_y = me.panel_x + me.facing, 1
    me.timer2 = 2
    me:set_action(8)
end

-- 0x02033444: Colonel's charge.
local function colonel_charge(me: Object)
    if double_attack.charge(me, CHARGE_TICKS) then
        me:set_action(8)
    end
end

-- 0x020337D8: down Colonel's row from the panel ahead, the first enemy body whose panel before it is solid and free:
-- that panel (x, y); (0, 0) for none.
local function search(me: Object): (number, number)
    local front = me.facing
    local y = me.panel_y
    local enemy = ENEMY_BODY[me.alliance]
    local taken = TAKEN[me.alliance]
    local x = me.panel_x + front
    while field.valid(x, y) do
        if field.check(x, y, enemy, 0) and field.check(x - front, y, STANDABLE, taken) then
            return x - front, y
        end
        x += front
    end
    return 0, 0
end

-- A wait of `ticks` ticks (its first tick setting it), then `next`; true when it ended.
local function wait(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.timer = ticks
        me.phase_init = 4
    end
    return double_attack.ran_out(me)
end

-- 0x020334D8: Colonel looks (both navis' future panels the panel found), 30 ticks; nothing found, he leaves.
local function colonel_look(me: Object)
    if me.phase_init == 0 then
        local x, y = search(me)
        me.future_panel_x, me.future_panel_y = x, y
        local p = double_attack.partner(me)
        p.future_panel_x, p.future_panel_y = x, y
        me.timer = LOOK_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me.phase, me.phase_init = if me.future_panel_x == 0 then 0x24 else 4, 0
    end
end

-- 0x020330E8: MegaMan waits as Colonel looks, 30 ticks; nothing found, he leaves.
local function megaman_look(me: Object)
    if wait(me, LOOK_TICKS) then
        me.phase, me.phase_init = if me.future_panel_x == 0 then 0x24 else 4, 0
    end
end

-- 0x02033510: Colonel throws his cape from the panel found (its sound), 15 ticks.
local function colonel_throw(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_THROW
        battle.play_sound(SOUND_THROW)
        me.timer = WIND_TICKS
        me.phase_init = 4
    end
    if me.timer == CAPE_AT then
        throw_cape(me, me.future_panel_x, me.future_panel_y)
    end
    if double_attack.ran_out(me) then
        me.phase, me.phase_init = 8, 0
    end
end

-- 0x02033114: MegaMan waits as Colonel throws, 15 ticks.
local function megaman_wind(me: Object)
    if wait(me, WIND_TICKS) then
        me.phase, me.phase_init = 8, 0
    end
end

-- 0x02033138, 0x02033558: each leaps off the field (4 ticks): panel (0, 0), 190 pixels up.
local function leap(anim: number): (me: Object) -> ()
    return function(me: Object)
        if me.phase_init == 0 then
            me.anim = anim
            me.timer = LEAP_TICKS
            me.phase_init = 4
        end
        if double_attack.ran_out(me) then
            me.panel_x, me.panel_y = 0, 0
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, AWAY_Z)
            me.phase, me.phase_init = 0xC, 0
        end
    end
end

-- 0x0203316C, 0x0203358C: each waits for Colonel's cape: gone, they leave; stopped on its catch, they drop.
local function await_cape(me: Object, colonel_navi: Object)
    local held = (colonel_navi.state :: State).cape
    if held == nil then
        me.phase, me.phase_init = 0x24, 0
    elseif cape_stopped(held) then
        me.phase, me.phase_init = 0x10, 0
    end
end

-- 0x0203318C: MegaMan drops onto the panel found, 10 pixels down from its middle (his role's offset), 4 ticks.
local function megaman_drop(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_LAND
        local p = double_attack.partner(me)
        me.panel_x, me.panel_y = p.future_panel_x, p.future_panel_y
        me:set_coordinates_from_panel()
        local pos = me.pos
        me.pos = Vec3.new(pos.x, pos.y + 10 * 0x10000, 0)
        me.timer = DROP_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me.phase, me.phase_init = 0x14, 0
    end
end

-- 0x020335A8: Colonel drops onto it, 10 pixels back from its middle (his role's offset), 4 ticks.
local function colonel_drop(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_COLONEL_LAND
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:set_coordinates_from_panel()
        local pos = me.pos
        me.pos = Vec3.new(pos.x - 10 * me.facing * 0x10000, pos.y, 0)
        me.timer = DROP_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me.phase, me.phase_init = 0x14, 0
    end
end

-- 0x020331CC, 0x020335EC: each stands ready, 15 ticks.
local function ready(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_IDLE
        me.timer = READY_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me.phase, me.phase_init = 0x18, 0
    end
end

-- MegaMan's shot: his buster arm and its muzzle flash.
local function shoot(me: Object, restarts_overlay: boolean)
    local s = me.state :: State
    me.anim, me.anim_loaded = ANIM_SHOOT, 0xFF
    if not restarts_overlay then
        battle.play_sound(SOUND_SHOT)
    end
    double_attack.detach(s.held)
    double_attack.raise_arm(me, "held")
    double_attack.detach(s.held2)
    attachment.attach(me, buster.flash, "held2", { anim = 0, while_dimmed = true })
    if restarts_overlay then
        -- sub_80C44D2
        local overlay = me.related2
        if overlay then
            overlay.anim_loaded = 0xFF
        end
        battle.play_sound(SOUND_SHOT)
    end
end

local function at(list: { number }, t: number): boolean
    return table.find(list, t) ~= nil
end

-- 0x020331F0: MegaMan's rush, 40 ticks: a shot, his three hits, two more shots; then 30 ticks.
local function megaman_rush(me: Object)
    if me.phase_init == 0 then
        shoot(me, false)
        me.timer = RUSH_TICKS
        me.phase_init = 4
    end
    if at(HITS_AT, me.timer) then
        hit(me)
    end
    if at(SHOTS_AT, me.timer) then
        shoot(me, true)
    end
    if double_attack.ran_out(me) then
        me.timer = FINISH_WAIT
        me.phase, me.phase_init = 0x1C, 0
    end
end

-- 0x02033610: Colonel's rush, 40 ticks: his cape let go, his three strikes (each shaking the cameras), their sounds;
-- then 30 ticks.
local function colonel_rush(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_RUSH
        battle.play_sound(SOUND_RUSH)
        me.timer = RUSH_TICKS
        local held = (me.state :: State).cape
        if held then
            (held.state :: CapeState).released = true
        end
        me.phase_init = 4
    end
    if at(HITS_AT, me.timer) then
        battle.shake_camera(0, 0xA)
        hit(me)
    end
    if at(SHOTS_AT, me.timer) then
        battle.play_sound(SOUND_RUSH)
    end
    if double_attack.ran_out(me) then
        me.timer = FINISH_WAIT
        me.phase, me.phase_init = 0x1C, 0
    end
end

-- 0x02033298: MegaMan's finisher: the wait out, then his sword (the cameras shake, the screen flashes), its hit 10
-- ticks in; 21 ticks.
local function megaman_finish(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        if not double_attack.ran_out(me) then
            return
        end
        me.anim = ANIM_SWING
        double_attack.detach(s.held)
        double_attack.detach(s.held2)
        attachment.attach(me, SWORD, "held", { anim = MEGAMAN_SWORD_ANIM, while_dimmed = true })
        battle.play_sound(SOUND_SWING)
        me.timer = FINISH_TICKS
        battle.shake_camera(3, 0xA)
        battle.palette_flash(0, 0xA, true, false)
        me.phase_init = 4
    end
    if me.timer == FINISHER_AT then
        hit(me)
    end
    if double_attack.ran_out(me) then
        me.phase, me.phase_init = 0x20, 0
    end
end

-- 0x02033674: Colonel's finisher: the wait out, then his sword, its hit (the cameras shaking harder) 10 ticks in; 21
-- ticks.
local function colonel_finish(me: Object)
    if me.phase_init == 0 then
        if not double_attack.ran_out(me) then
            return
        end
        me.anim = ANIM_FINISH
        attachment.attach(me, SWORD, "held", { anim = COLONEL_SWORD_ANIM, while_dimmed = true })
        battle.play_sound(SOUND_FINISH)
        me.timer = FINISH_TICKS
        me.phase_init = 4
    end
    if me.timer == FINISHER_AT then
        battle.shake_camera(2, 0x14)
        hit(me)
    end
    if double_attack.ran_out(me) then
        me.phase, me.phase_init = 0x20, 0
    end
end

-- 0x0203330C, 0x020336CC: 20 ticks; then the sword let go (its slot cleared), and Colonel's hold of his cape ends.
local function done(me: Object)
    if wait(me, AFTER_FINISH) then
        local s = me.state :: State
        s.held = nil
        s.cape = nil
        me.phase, me.phase_init = 0x24, 0
    end
end

-- 0x02033330, 0x020336F0: 4 ticks of his leaving animation, hidden, then the action after.
local function out(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_LEAP
        me.timer = OUT_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me.visible = false
        me:set_action(0xC)
    end
end

-- The rush, by phase (0x020DDDCC's MegaMan's, 0x020DDE08's Colonel's).
local function phases(table_: { [number]: (me: Object) -> () }, who: string): (me: Object) -> ()
    return function(me: Object)
        local f = table_[me.phase]
        if not f then
            error(string.format("BR-RUSH's %s: phase %#x reads past his table", who, me.phase))
        end
        f(me)
    end
end

local megaman_phases = phases({
    [0x0] = megaman_look,
    [0x4] = megaman_wind,
    [0x8] = leap(ANIM_LEAP),
    [0xC] = function(me: Object)
        await_cape(me, double_attack.partner(me))
    end,
    [0x10] = megaman_drop,
    [0x14] = ready,
    [0x18] = megaman_rush,
    [0x1C] = megaman_finish,
    [0x20] = done,
    [0x24] = out,
}, "MegaMan")

local colonel_phases = phases({
    [0x0] = colonel_look,
    [0x4] = colonel_throw,
    [0x8] = leap(ANIM_COLONEL_LEAP),
    [0xC] = function(me: Object)
        await_cape(me, me)
    end,
    [0x10] = colonel_drop,
    [0x14] = ready,
    [0x18] = colonel_rush,
    [0x1C] = colonel_finish,
    [0x20] = done,
    [0x24] = out,
}, "Colonel")

-- 0x02033380, 0x02033744: 30 ticks, then he leaves.
local function after(me: Object)
    if wait(me, AFTER_TICKS) then
        me:set_action(0x10)
    end
end

local function leave(me: Object)
    double_attack.leave(me, true)
end

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "special",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 133,
    lockout = 0,
    flags = {},
    action = cannon_chips.cannon.action,
    double_attack = double_attack.make {
        id = "br-rush",
        damage = { 10, 12, 15, 15, 20, 25, 35, 45, 50, 50 },
        state = { cape = "object" },
        roles = {
            -- 0x02032FDC's table: MegaMan, (10, 10) from the user's spot.
            {
                navi = megaman,
                dx = 10,
                dy = 10,
                actions = { [0x0] = appear, [0x4] = megaman_charge, [0x8] = megaman_phases, [0xC] = after, [0x10] = leave },
            },
            -- 0x020333D8's: Colonel, (-10, -10).
            {
                navi = colonel,
                dx = -10,
                dy = -10,
                actions = {
                    [0x0] = colonel_appear,
                    [0x4] = colonel_charge,
                    [0x8] = colonel_phases,
                    [0xC] = after,
                    [0x10] = leave,
                },
            },
        },
    },
}

return { ["br-rush"] = chip }
