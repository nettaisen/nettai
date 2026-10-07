--!strict
-- A boulder, as EXE5 and EXE6 share it (EXE6's attack object #0x6E, `sub_80D2290`; EXE5's #0x6E, 0x080D38C0, the same
-- code): the field obstacle some stages start with (the actor lists' entry type 3, EXE6's `sub_8007450` and EXE5's
-- 0x080068A0: the kind's `place`), 500 HP, on its panel's side. It stands through the round's start and takes its
-- place once the fight is on; it has no lifetime. Broken, it leaves two chunks of rock debris and a splash of dust;
-- a chip can remove (or, in EXE6, absorb) it. It is one of the field's two stage objects (the field-object registry's
-- stage slots, which it leaves when it goes). See EXE6's docs/engine/field-objects.md §3.1.
--
-- Each game makes its own kind (`boulder.make`) with its look: the sprite, its identity, the debris and effects it
-- leaves, how it looks absorbed (EXE6's; EXE5 neither absorbs nor blinks obstacles out: a removal leaves a puff,
-- 0x080D39C4) and the damage it is thrown for (EXE6's 200, EXE5's 100: 0x080068D0). EXE4's (attack object #0x6E,
-- 0x080DB0EC) is the same code but for what its look states besides: no identity, its hits never pushed and its own
-- reaction, the action it keeps once the fight is on, its own action table, and a leaving by its HP alone.
--
-- An obstacle: its update runs the obstacle framework's steps (the `obstacle` service) around its own actions.

local debris = require("./rock/debris")

local boulder = {}

export type Look = {
    -- The kind's id.
    id: string,
    sprite: SpriteAsset,
    -- What it is taken for: a field object, with the look something that takes its place shows (DustMan's junk); none,
    -- its init writes no NameID (EXE4's 0x080DB10C).
    identity: Identity?,
    -- The game's debris kind (@exelib/rock/debris), and the effects it leaves: a puff when removed, a splash of
    -- dust when broken.
    debris: Kind,
    effects: { puff: Effect, splash: Effect },
    -- What it is and reaches (a neutral body that everything reaches, while dimmed too).
    is: CollisionType,
    hits: CollisionType,
    -- How it looks absorbed (EXE6's obstacle kind 5: its own sprite); none (EXE5's): a removal leaves a puff.
    absorbed: AbsorbedLook?,
    -- The damage it does when thrown (its spawner's damage word).
    damage: number,
    -- How it takes its hits and reacts (none: the framework's; EXE4's 0x080139A0 never pushed, and 0x08014058's
    -- reaction: destroyed by an obstacle's touch or a breaking hit but no body's, the HP as it is).
    push: ObstaclePush?,
    crush: ObstacleCrush?,
    -- Once the fight is on it keeps its waiting action (EXE4's 0x080DB1BC); none: it stands.
    keeps_waiting: boolean?,
    -- Its action table by the framework's numbers, given the shared waiting, leaving and standing (EXE4's
    -- 0x080DB1A0); none: EXE6's `jt_80D2348`.
    actions: ((wait: (Object) -> (), destroyed: (Object) -> (), standing: (Object) -> ()) -> { [number]: (Object) -> () })?,
    -- It leaves releasing no tracking, by its HP alone: left, a puff; none, the debris and the splash (EXE4's
    -- 0x080DB1EC); none: by how it was removed.
    leaves_by_hp: boolean?,
}

local HP = 500
-- What its timer starts at (nothing counts it down).
local TIMER = 0x1770
local HIT_MOD = 3
-- The palette of the debris it breaks into.
local DEBRIS_PALETTE = 0
-- A panel of side 1's.
local SIDE_1 = 0x20
-- The header flags the spawner (EXE6's `sub_80D2430`, EXE5's 0x080D3A3E) leaves. It means to add the pause and
-- dimming bits (0x14) to the new object's flags, but reads the flags through the panel's column instead of the
-- object: an address in the console's BIOS, which game code can't read. What comes back is a byte of the
-- instruction word the BIOS last fetched (after a software interrupt, which the game's divisions are,
-- 0xE3A02004), by the address's low two bits; the object's own flags (in use, sprite not animating) are lost. On
-- columns 2 and 5, where the stages' boulders are, that leaves it "not in use" (its sprite never steps) with stray
-- bits (holds a reservation; on column 2 also 0x80, which nothing reads).
local BIOS_WORD, RUNS_PAUSED_AND_DIMMED = 0xE3A02004, 0x14

-- Its actions (EXE6's `jt_80D2348`; EXE5's table has no frozen or bubbled rows and stands in its 6, the same
-- actions).
local WAIT, DESTROYED, STANDING = 0, 2, 8

-- `sub_80D236C`: once the fight is on, it stands (or keeps waiting), and no longer runs while the battle is paused.
local function waiting(keeps: boolean): (me: Object) -> ()
    return function(me: Object)
        if not battle.fighting() then
            return
        end
        me.run_while_paused = false
        me.visible = true
        me:set_action(obstacle.action_byte(me, if keeps then WAIT else STANDING))
    end
end

-- `sub_80D2398`.
local function standing(_me: Object) end

function boulder.make(look: Look): Kind
    local absorbed = look.absorbed
    local by_hp = look.leaves_by_hp == true
    local wait = waiting(look.keeps_waiting == true)

    -- `sub_80D239C`: leave the field and its stage slot: absorbed, removed by a chip (a puff, or blinking out), or
    -- broken into two chunks and dust (or by its HP alone, `leaves_by_hp`).
    local function destroyed(me: Object)
        if not by_hp then
            obstacle.release_tracking(me)
        end
        me.collision:set_region(nil)
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        obstacle.leave_stage(me)
        local how = if by_hp then (if me.hp ~= 0 then "removed" else "broken") else obstacle.removal(me)
        if by_hp and how == "removed" then
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), look.effects.puff)
        elseif absorbed and how == "absorbed" then
            obstacle.fly_to_absorber(me, absorbed)
        elseif how ~= "broken" then
            local blink = if absorbed then obstacle.blink_out(me) else "no"
            if blink == "blinking" then
                return
            elseif blink == "no" then
                local p = me.pos
                battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), look.effects.puff)
            end
        else
            local p = me.pos
            for _ = 1, 2 do
                debris.spawn(look.debris, battle.jitter(0xF, p), DEBRIS_PALETTE)
            end
            battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), look.effects.splash)
        end
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end

    -- The boulder's action table (`jt_80D2348`): its own actions and the framework's shared ones (or its look's).
    local own = look.actions
    local ACTIONS: { [number]: (Object) -> () } = if own
        then own(wait, destroyed, standing)
        else {
            [WAIT] = wait,
            [1] = obstacle.return_to_idle,
            [DESTROYED] = destroyed,
            [3] = obstacle.flinch,
            [4] = obstacle.paralyzed,
            [5] = obstacle.slide,
            [6] = obstacle.frozen,
            [7] = obstacle.bubbled,
            [STANDING] = standing,
        }

    -- `sub_80D2320`: hits, the reaction, its action; no hit spark and no lifetime. Its sprite steps while dimmed
    -- too.
    local function tick(me: Object)
        me.visible = true
        obstacle.take_hits(me, look.push)
        local a = obstacle.react(me, look.crush)
        if a then
            local action = ACTIONS[a]
            if not action then
                error(string.format("boulder action %d reads past its table", a))
            end
            action(me)
        end
        me.collision:present()
        me:update_sprite_while_dimmed()
    end

    -- `sub_80D22B0`: on its panel, a pixel back and a pixel down (so what stands on the panel's center is drawn over
    -- it), with its collision on the panel at once. Its first update is the next tick's.
    local function init(me: Object)
        local sprite = me.sprite
        sprite:load(look.sprite)
        me.anim, me.anim_loaded = 0, 0
        sprite:set_animation(0)
        sprite:step()
        sprite.shadow = "ground"
        me:set_coordinates_from_panel()
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y - 0x1_0000, -0x1_0000)
        if look.identity then
            me.identity = look.identity
        end
        me.hp, me.max_hp = HP, HP
        me.timer = TIMER
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(look.is, look.hits, HIT_MOD)
        me.collision:present()
        me.lifecycle = "update"
    end

    -- The kind (its `place` spawns one).
    local kind: Kind
    kind = new.kind {
        id = look.id,
        pool = "attack",
        -- `sub_80D2290`.
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
        -- `sub_8007450`: a stage's boulder (an actor list's entry type 3), on its panel's side, if one of the
        -- field's two stage slots is free; running while the battle is paused or dimmed, with the flags its
        -- spawner's bug leaves. (`sub_80D2430`; until its init places it, its position is the spawner's registers:
        -- Y and what nothing reads.)
        place = function(spec: PlaceSpec): Object?
            if not obstacle.stage_slot_free() then
                return nil
            end
            local x, y = spec.panel_x, spec.panel_y
            local o = battle.spawn(kind, Vec3.new(y, 0, 0))
            if not o then
                return nil
            end
            o.panel_x, o.panel_y = x, y
            o:set_damage_word(look.damage)
            o.element = 0
            o.alliance = if bit32.btest(field.flags(x, y), SIDE_1) then 1 else 0
            local read = bit32.band(bit32.rshift(BIOS_WORD, 8 * (x % 4)), 0xFF)
            o:set_header_flags(bit32.bor(read, RUNS_PAUSED_AND_DIMMED))
            obstacle.enter_stage(o)
            return o
        end,
    }
    return kind
end

return boulder
