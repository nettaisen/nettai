--!strict
-- EXE5's BoyBomb1-3 (the dimming handler's subtype 58): EXE6's leftover code (EXE6's effect object #0x67, `sub_80E7DC4`,
-- and attack object #0x3E, `sub_80CBB44`, which no EXE6 chip spawns) with EXE5's numbers. Its controller (effect object
-- #0x67, 0x080E8008: the usual dimming phases) sets a bomb on the panel ahead of the user if it is free, and waits 30
-- ticks. The bomb (attack object #0x3E, 0x080CCDF4) fades in over 16 ticks, then, on a panel it may stand on, counts
-- down 60 ticks blinking and blows up over the 3x3 around it (no spark, against navis, hit modifier 3); on a panel it
-- may not, it breaks. An obstacle of 150 HP, it takes hits (any one side's hit pushes it a panel away: EXE5's
-- reaction by the hitters' flips, 0x08017A78), is pushed and breaks as one (the obstacle framework, in its numbering:
-- its standing the framework's 8, which EXE5 stores as 6: `obstacle.action_byte`).
--
-- The bomb's Param1 0 (HP 50, a body against the other side's, exploding over the whole enemy area, kept on a
-- holder's record: 0x08108274's, an AI's) is no chip's: this module spawns its Param1 1 alone.

local collision = require("../../rules/collision")
local dimming_chips = require("@exelib/dimming")
local regions = require("@exelib/regions")

local bomb = {}

export type State = {
    -- Param2: set when it stands where it may not (0x080CD29C): it breaks.
    doomed: boolean,
    -- +0x60: its HP changed (20 ticks), and +0x64: the HP it last saw.
    hit: boolean,
    last_hp: number,
}

export type ControllerState = {
    -- The Atk+ bonus (+0x32): it adds to the bomb's damage word.
    bonus: number,
}

-- 0x080CCE26: sprite 10-36 without a shadow, animation 0.
local SPRITE = asset.sprite("boy-bomb")
local HP = 150
-- Its collision (Param1 1): a neutral body (row 0x0E) that everything reaches (row 0x0F), hit modifier 3, no spark.
local HIT_MOD = 3
-- NameID 0xD4: actor record 0xD4 (0x08014C94: version 1, AI index 0x21).
local IDENTITY = define.identity {
    id = "boy-bomb",
    class = "field_object",
    version = 1,
    ai_index = 0x21,
}
-- The effects (EXE5's table, 0x080DFC94): row 6 (0C-12) as it comes, row 0 (14-00) over the blast, row 0x14 (14-0D,
-- animation 2) when removed, row 0x0E (14-01) when broken.
local COMES = define.effect { sprite = asset.sprite("heal"), anim = 0, palette = 0 }
local BLAST = define.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
local REMOVED = define.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }
local BROKEN = define.effect { sprite = asset.sprite("impact"), anim = 0, palette = 0 }
local PLACE_SOUND, BLAST_SOUND = asset.sound("place"), asset.sound("target-move")
local BREAK_SOUND = asset.sound("hit-bomb-1")
local FADE_TICKS, COUNTDOWN, HIT_TICKS, WAIT = 15, 60, 20, 30
-- Its countdown's blink (0x080CD08C): the color shader by the timer's bit 2.
local BLINK = { [0] = 0x00, 0x12 }
-- Where it may stand (0x080CD2BC, by its side): solid, without its side's navi or another body.
local SOLID = 0x10
local FORBID = { [0] = 0x0B800000, [1] = 0x06800000 }
-- Where the controller sets it: solid with nothing on it.
local OCCUPIED = 0x0F880080

-- The actions (0x080CCF3C: EXE5's seven, here by EXE6's numbers).
local APPEAR, DESTROYED, IDLE = 0, 2, 8

local function state(me: Object): State
    return me.state :: State
end

-- 0x080CD29C: whether it may stand where it is (else it breaks: Param2).
local function may_stand(me: Object): boolean
    local ok = field.check(me.panel_x, me.panel_y, SOLID, FORBID[bit32.band(me.alliance, 1)])
    if not ok then
        state(me).doomed = true
    end
    return ok
end

-- 0x080CD222: the countdown starts.
local function start_countdown(me: Object)
    me.timer = COUNTDOWN
    me.phase, me.phase_init = 4, 0
    me:set_status("moving", false)
    me:set_status("move_complete", true)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
end

-- 0x080CCF58: it fades in (the alpha its second timer counts), 16 ticks; then it stands, and on a panel it may stand
-- on its region is its panel, its reservation goes and the countdown starts.
local function appear(me: Object)
    local t2 = int.u16(me.timer2 + 1)
    me.timer2 = t2
    me.sprite.alpha = t2
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    me.sprite.alpha = nil
    -- (The action byte alone: a store meant for the phase goes to address 0x0A.)
    me.action = obstacle.action_byte(me, IDLE)
    if may_stand(me) then
        me.collision:set_region(regions.single)
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        start_countdown(me)
    end
end

-- 0x080CCFB8: standing: its hit mark counts down, it follows where it is pushed, and it checks where it stands.
local function stand(me: Object)
    local s = state(me)
    if s.hit then
        local left = int.u16(me.timer2 - 1)
        me.timer2 = left
        if left == 0 then
            s.hit = false
        end
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    may_stand(me)
end

-- 0x080CCFDA: the countdown, blinking; at its end the blast over the 3x3 around it, and it is gone.
local function count_down(me: Object)
    me.sprite.color_shader = BLINK[bit32.rshift(bit32.band(me.timer, 4), 2)]
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left ~= 0 then
        return
    end
    local word = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = 0,
        z = 0,
        region = regions.square,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = bit32.band(word, 0xFFFF),
        stamina = bit32.rshift(word, 16),
        hit_mod = HIT_MOD,
    })
    battle.region_effects(me.panel_x, me.panel_y, regions.square, me.alliance, BLAST, 0)
    battle.play_sound(BLAST_SOUND)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080CCF9C: standing (phase 0), or counting down (4).
local function idle(me: Object)
    local p = me.phase
    if p == 0 then
        stand(me)
    elseif p == 4 then
        count_down(me)
    else
        error(string.format("BoyBomb's bomb phase %#x reads past its table", p))
    end
end

-- 0x080CD08E: it goes: a puff when a chip removed it, else it breaks (a sound, a spark).
local function destroyed(me: Object)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    local p = me.pos
    if obstacle.removal(me) == "removed" then
        battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), REMOVED)
    else
        battle.play_sound(BREAK_SOUND)
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x8_0000), BROKEN)
    end
    me.collision:set_region(nil)
    obstacle.unregister(me)
    obstacle.release_tracking(me)
    me:set_lifecycle("destroy")
end

local ACTIONS: { [number]: (Object) -> () } = {
    [APPEAR] = appear,
    [1] = obstacle.return_to_idle,
    [DESTROYED] = destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [5] = obstacle.knocked_back,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [IDLE] = idle,
}

-- 0x080CCEE8.
local function tick(me: Object)
    local s = state(me)
    -- (sprite_clearFinalPalette: the hits' resolution clears it too.)
    if me.hp ~= s.last_hp then
        s.last_hp = me.hp
        s.hit = true
        me.timer2 = HIT_TICKS
    end
    if not battle.dimmed() and (s.doomed or battle.over()) then
        me.hp = 0
    end
    obstacle.take_hits(me, "any_hit")
    local a = obstacle.react(me)
    if a then
        local action = ACTIONS[a]
        if not action then
            error(string.format("BoyBomb's bomb action %d reads past its table", a))
        end
        action(me)
    end
    me:update_sprite()
    me.collision:present()
end

-- 0x080CCE26: on its panel, its sprite, name and HP, its collision (no spark, no region yet), unpushable, fading in;
-- the effect and the sound as it comes; its panel reserved.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "hidden"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.identity = IDENTITY
    me.hp, me.max_hp = HP, HP
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.neutral_body, collision.everything, HIT_MOD)
    local c = me.collision
    -- (Its region byte set to 1, which the clear below undoes.)
    c:set_hit_effect(nil)
    c:set_region(nil)
    c:present()
    me:set_status("moving", true)
    me.timer, me.timer2 = FADE_TICKS, 0
    sprite.alpha = 0
    battle.effect(me.pos, COMES)
    battle.play_sound(PLACE_SOUND)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me:reserve_panel(me.panel_x, me.panel_y)
    me:set_lifecycle("update")
end

bomb.kind = define.kind {
    id = "boybomb/bomb",
    pool = "attack",
    state = { doomed = "bool", hit = "bool", last_hp = "u16" },
    -- 0x080CCDF4.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080CCE14 (its holder's record, a Param1 0 bomb's, isn't kept here).
            me:destroy()
        end
    end,
}

-- 0x080CD0E0: the bomb of `controller`'s on (x, y), with its element and damage word, running while dimmed, one of
-- its side's field objects. (It spawns at the registers' values, which its init replaces but for Z.)
local function spawn(controller: Object, x: number, y: number, element: number, damage: number)
    local o = battle.spawn(bomb.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = controller.alliance, controller.flip
        o.run_while_dimmed = true
        obstacle.register(o, o.alliance, 0)
    end
end

-- 0x080E804C: the bomb on the panel ahead if it is free (0x080E80A8), with the controller's damage word and bonus;
-- then 30 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        local x, y = me.panel_x + me.facing, me.panel_y
        if field.check(x, y, SOLID, OCCUPIED) then
            spawn(me, x, y, me.element, dimming_chips.damage_plus(me, (me.state :: ControllerState).bonus))
        end
        me.timer = WAIT
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left == 0 then
        dimming_chips.done(me)
    end
end

bomb.controller = define.kind {
    id = "boybomb/controller",
    pool = "effect",
    state = { bonus = "u16" },
    -- 0x080E8008.
    update = dimming_chips.phases { name = "BoyBomb", effect = effect },
}

-- 0x080E8086: the chips' `dimming` hook, the controller with the dimming's bonus. Its position is the dimming
-- handler's registers: the user's row, the element and the hook's own address (the handler's `bx r3`, Thumb).
local HOOK_ADDRESS = 0x080E8087

function bomb.hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(bomb.controller, user, spec)
    if o then
        o.pos = Vec3.new(user.panel_y, spec.element, HOOK_ADDRESS);
        (o.state :: ControllerState).bonus = spec.bonus
    end
    return o
end

return bomb
