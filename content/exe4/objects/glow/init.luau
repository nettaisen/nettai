--!strict
-- EXE4's glow over a navi (actor object #0x5D, 0x080CCF5C): a copy of the side's navi, which vanishes under it, glowing
-- white. EXE6's and EXE5's (@exelib/bugfix/glow) but at its start, in its tick and at its end: EXE4's own. What it
-- does is its user's (the original's first parameter, 0x080CD034's rows: 0 BugFix's flashes, 0x080CD03C; 1 PopUp's
-- dive, 0x080CD0E2): each user makes its own kind with its own action (`glow.make`), the original's one actor kind
-- compat's. When it is done it clears its holder's `busy` flag (the byte its spawner gave it, 0x080CD14E).
--
-- Its start (0x080CCF7C): only a navi's copy (the navi's +0x0F 0, MegaMan's, or 0xF0 to 0xFC, a navi's in the story;
-- anything else, a virus, it isn't busy over and goes at once); the navi's sprite by its navi and soul (0x0800B90A,
-- the engine's `load_like`), with its shadow (navi 10's, JunkMan's, without: the story's, never a netbattle's), a pixel
-- up the field and lower than the navi (as EXE5's), the navi's palette and its side's facing; what the navi's form
-- wears (0x0800B812: MegaMan's soul's overlay, 0x08018068 by the soul, stepping on, its Param3 1; another navi's own),
-- through the form's `put_on` (FireSoul's and NumberSoul's are docs/design/exe4-map.md §18 item 74's), taken off as
-- it goes (0x0800B5DA). Its tick (0x080CD01C): its action, then its sprite, while dimmed too; unlike EXE6's it
-- neither shows itself nor hides from a blind viewer.

local glow = {}

export type State = {
    -- The object whose `busy` it clears when done (the original's pointer, its +0x60).
    holder: Object?,
}

export type Spec = {
    -- The kind's id.
    id: string,
    -- Its action, each tick (its user's row of 0x080CD034): ends with `glow.back` and `glow.done`.
    act: (me: Object) -> (),
}

export type Glow = {
    kind: Kind,
    -- 0x080CD14E: a glow over `spawner`'s side's navi, on its side (not its facing); `holder` (an object with a
    -- boolean `busy`) is busy until it is done. (It is spawned at the spawner's registers; its init places it on the
    -- navi at once.)
    spawn: (spawner: Object, holder: Object) -> Object?,
}

-- The navi it copies (its first related object: the side's player).
function glow.navi_of(me: Object, routine: string): Object
    local navi = me.related1
    if not navi then
        error(string.format("the glow has no navi (%s reads through a null pointer)", routine))
    end
    return navi
end

-- Its holder isn't busy.
local function release(me: Object)
    local holder = (me.state :: State).holder
    if holder then
        (holder.state :: { busy: boolean }).busy = false
    end
end

-- The end its actions share (0x080CD0BE, 0x080CD114): it vanishes, what it wears comes off (0x0800B5DA, while it wears
-- something), and the navi is back (0x080E2DCC).
function glow.back(me: Object)
    me.visible = false
    local navi = glow.navi_of(me, "0x080E2DCC")
    if me.related2 then
        me:take_off_form_overlay(battle.navi(me.alliance).form)
    end
    dimming.show_user(navi)
end

-- Done: its holder isn't busy, and it goes.
function glow.done(me: Object)
    release(me)
    me:set_lifecycle("destroy")
end

function glow.make(spec: Spec): Glow
    -- 0x080CD01C.
    local function tick(me: Object)
        spec.act(me)
        me:update_sprite_while_dimmed()
    end

    -- 0x080CCF7C.
    local function init(me: Object)
        local navi = glow.navi_of(me, "0x080CCF7C")
        if navi.actor_type == "virus" then
            -- 0x080CD010.
            release(me)
            me:free()
            return
        end
        local sprite = me.sprite
        sprite:load_like(navi)
        sprite.shadow = "ground"
        me.visible = true
        local p = navi.pos
        me.pos = Vec3.new(p.x, p.y - 0x10000, p.z - 0x10000)
        -- 0x0800BFFE: the navi's palette (the one it wears).
        sprite.palette = navi.sprite.palette
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me:put_on_form_overlay(battle.navi(me.alliance).form)
        me.anim, me.anim_loaded = 0, 0xFF
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = spec.id,
        pool = "actor",
        state = { holder = "object" },
        -- 0x080CCF5C.
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

    local function spawn(spawner: Object, holder: Object): Object?
        local o = battle.spawn(kind, Vec3.zero)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.holder = holder;
        (holder.state :: { busy: boolean }).busy = true
        o.alliance = spawner.alliance
        o.related1 = battle.player(spawner.alliance)
        return o
    end

    return { kind = kind, spawn = spawn }
end

return glow
