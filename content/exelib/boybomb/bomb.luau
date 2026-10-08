--!strict
-- BoyBomb's bomb (attack object #0x3E in the three games: EXE6's `sub_80CBB44`, which no EXE6 chip spawns; EXE5's
-- 0x080CCDF4; EXE4's 0x080D4F40), made for a game's look (`bomb.make`). It fades in over 16 ticks, then, on a panel
-- it may stand on, counts down 60 ticks blinking and blows up over the 3x3 around it (no spark, against navis, hit
-- modifier 3); on a panel it may not, it breaks. An obstacle of 150 HP, it takes hits, is pushed and breaks as one
-- (the obstacle framework, in its numbering: its standing the framework's 8, which the games store as 6:
-- `obstacle.action_byte`).
--
-- The games part in its look: its sprite, NameID, effects and sounds; where it may stand, by its side; how it takes
-- hits; the game's obstacle rows (1, 3 to 7); and EXE5's additions EXE4 lacks: its panel reserved until it stands,
-- the tracking it releases as it goes, its removal by a chip (a puff), and the dimming keeping it whole
-- (`breaks_while_dimmed`: EXE4's drops its HP for a bad panel or the battle's end even while dimmed).
--
-- The bomb's Param1 0 (a body against the other side's, exploding over the whole enemy area, kept on a holder's
-- record: an AI's) is no chip's: the maker spawns its Param1 1 alone.

local regions = require("../regions")

local bomb = {}

export type State = {
    -- Param2: set when it stands where it may not (EXE5's 0x080CD29C): it breaks.
    doomed: boolean,
    -- +0x60: its HP changed (20 ticks), and +0x64: the HP it last saw.
    hit: boolean,
    last_hp: number,
}

local HP = 150
-- Its collision (Param1 1): a neutral body that everything reaches, hit modifier 3, no spark.
local HIT_MOD = 3
local FADE_TICKS, COUNTDOWN, HIT_TICKS = 15, 60, 20
-- Its countdown's blink (EXE5's 0x080CD08C, EXE4's 0x080D51AC): the color shader by the timer's bit 2.
local BLINK = { [0] = 0x00, 0x12 }
-- Where it may stand: solid, without what its side's entry forbids.
local SOLID = 0x10

-- The actions, by the framework's numbers.
local APPEAR, DESTROYED, IDLE = 0, 2, 8

local function state(me: Object): State
    return me.state :: State
end

-- `look`'s bomb: its kind and spawn.
function bomb.make(look: BoyBombLook): BoyBomb
    -- EXE5's 0x080CD29C: whether it may stand where it is (else it breaks: Param2).
    local function may_stand(me: Object): boolean
        local ok = field.check(me.panel_x, me.panel_y, SOLID, look.stand_forbid[bit32.band(me.alliance, 1)])
        if not ok then
            state(me).doomed = true
        end
        return ok
    end

    -- EXE5's 0x080CD222: the countdown starts.
    local function start_countdown(me: Object)
        me.timer = COUNTDOWN
        me.phase, me.phase_init = 4, 0
        me:set_status("moving", false)
        me:set_status("move_complete", true)
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    end

    -- EXE5's 0x080CCF58: it fades in (the alpha its second timer counts), 16 ticks; then it stands, and on a panel it
    -- may stand on its region is its panel, its reservation goes and the countdown starts.
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
            if look.reserves_panel then
                me:unreserve_panel(me.future_panel_x, me.future_panel_y)
            end
            start_countdown(me)
        end
    end

    -- EXE5's 0x080CCFB8: standing: its hit mark counts down, it follows where it is pushed, and it checks where it
    -- stands.
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

    -- EXE5's 0x080CCFDA: the countdown, blinking; at its end the blast over the 3x3 around it, and it is gone.
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
            target = look.blast.hits,
            self_type = look.blast.is,
            damage = bit32.band(word, 0xFFFF),
            stamina = bit32.rshift(word, 16),
            hit_mod = HIT_MOD,
        })
        battle.region_effects(me.panel_x, me.panel_y, regions.square, me.alliance, look.blast.effect, 0)
        battle.play_sound(look.blast.sound)
        if look.releases_tracking then
            obstacle.release_tracking(me)
        end
        obstacle.unregister(me)
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end

    -- EXE5's 0x080CCF9C: standing (phase 0), or counting down (4).
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

    -- EXE5's 0x080CD08E: it goes: a puff when a chip removed it (EXE5's), else it breaks (a sound, a spark).
    local function destroyed(me: Object)
        if look.reserves_panel then
            me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        end
        local p = me.pos
        local removed = look.removed
        if removed and obstacle.removal(me) == "removed" then
            battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), removed)
        else
            battle.play_sound(look.break_sound)
            battle.effect(Vec3.new(p.x, p.y, p.z + 0x8_0000), look.broken)
        end
        me.collision:set_region(nil)
        obstacle.unregister(me)
        if look.releases_tracking then
            obstacle.release_tracking(me)
        end
        me:set_lifecycle("destroy")
    end

    local actions: { [number]: (Object) -> () } = { [APPEAR] = appear, [DESTROYED] = destroyed, [IDLE] = idle }
    for a, f in look.actions do
        actions[a] = f
    end

    -- EXE5's 0x080CCEE8.
    local function tick(me: Object)
        local s = state(me)
        -- (sprite_clearFinalPalette: the hits' resolution clears it too.)
        if me.hp ~= s.last_hp then
            s.last_hp = me.hp
            s.hit = true
            me.timer2 = HIT_TICKS
        end
        if (look.breaks_while_dimmed or not battle.dimmed()) and (s.doomed or battle.over()) then
            me.hp = 0
        end
        obstacle.take_hits(me, look.push)
        local a = obstacle.react(me)
        if a then
            local action = actions[a]
            if not action then
                error(string.format("BoyBomb's bomb action %d reads past its table", a))
            end
            action(me)
        end
        me:update_sprite()
        me.collision:present()
    end

    -- EXE5's 0x080CCE26: on its panel, its sprite, name and HP, its collision (no spark, no region yet), unpushable,
    -- fading in; the effect and the sound as it comes; its panel reserved (EXE5's).
    local function init(me: Object)
        me:set_coordinates_from_panel()
        local sprite = me.sprite
        sprite:load(look.sprite)
        -- (`sprite_noShadow`: the first part is drawn with the sprite.)
        sprite.shadow = "with_sprite"
        me.visible = true
        me.anim, me.anim_loaded = 0, 0
        sprite:set_animation(0)
        sprite.palette = 0
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        if look.identity then
            me.identity = look.identity
        end
        me.hp, me.max_hp = HP, HP
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(look.body.is, look.body.hits, HIT_MOD)
        local c = me.collision
        -- (Its region byte set to 1, which the clear below undoes.)
        c:set_hit_effect(nil)
        c:set_region(nil)
        c:present()
        me:set_status("moving", true)
        me.timer, me.timer2 = FADE_TICKS, 0
        sprite.alpha = 0
        battle.effect(me.pos, look.comes)
        battle.play_sound(look.place_sound)
        if look.reserves_panel then
            me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
            me:reserve_panel(me.panel_x, me.panel_y)
        end
        me:set_lifecycle("update")
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        state = { doomed = "bool", hit = "bool", last_hp = "u16" },
        -- EXE5's 0x080CCDF4.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                -- EXE5's 0x080CCE14 (its holder's record, a Param1 0 bomb's, isn't kept here).
                me:destroy()
            end
        end,
    }

    -- EXE5's 0x080CD0E0: the bomb of `controller`'s on (x, y), with its element and damage word, running while
    -- dimmed, one of its side's field objects (class 0); its side and facing the controller's (EXE4's its side
    -- alone, `side_only`). (It spawns at the registers' values, which its init replaces but for Z.)
    local function spawn(controller: Object, x: number, y: number, element: number, damage: number): Object?
        local o = battle.spawn(kind, Vec3.new(y, element, 0))
        if o then
            o.panel_x, o.panel_y = x, y
            o.element = element
            o:set_damage_word(damage)
            o.alliance = controller.alliance
            if not look.side_only then
                o.flip = controller.flip
            end
            o.run_while_dimmed = true
            obstacle.register(o, o.alliance, 0)
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return bomb
