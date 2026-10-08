--!strict
-- NumberMan's face, as EXE5 and EXE4 share it (actor object #0x5C: EXE5's 0x080C412E, EXE4's 0x080CCC9C; their own,
-- the same code but for its start; NumberMan's, its spawner's Param0 0 and Param2 1, and EXE4's AquaMan's spout and
-- WindMan's swirl, Param0 1 and 2 (its sprite rows 1 and 2) and Param2 1): a second sprite on him, following his
-- position, visibility, color shader, alpha and palette. Once it follows, it is drawn while he stands (his animation
-- 0), restarting its animation each time he comes back to it, and hidden otherwise. He lets it go when he leaves.
-- (Param0 2 and 3 follow by 0x080CCEF4 (0x080CCEBC's table): the same, but setting its animation each tick, 1 while
-- the object at the word its +0x60 points at has Param1 0, else 0; WindMan points it at his +0x64, which he never
-- sets, so his swirl's stays 0, as rows 0 and 1 keep theirs.)
--
-- Its start: EXE5's waits for every navi to be in, running while paused till then, and moves on to its follow (its
-- action 4) at its next update. EXE4's (0x080CCD0A) waits so only with the fight off; with the fight on, as a navi
-- chip always finds it, it follows (its action 8) from its first update. (EXE4's action 4 is a random battle's
-- intro, which nettai's link battles never reach.)
--
-- Not reached: Param0 2 and 3 with a watched object (EXE4's 0x0800DB76's) and EXE5's sprites past its table's first
-- row (0x080C412C). Each user states its sprite (EXE4's row by Param0, 0x080CCC94).
--
-- Each game makes its kind (`face.make`).

local face = {}

type State = {
    -- It was hidden while its owner moved (+0x0C): its animation restarts when he stands again.
    hidden: boolean,
}

local WAIT = 0

function face.make(spec: NumberManFaceSpec): NumberManFace
    local follows = if spec.follows_in_fight then 8 else 4

    -- EXE5's 0x080C419C, EXE4's 0x080CCD3C: on its owner, wearing his look; then its action.
    local function update(me: Object)
        local owner = me.related1 :: Object
        me.pos = owner.pos
        me.visible = owner.visible
        local look, worn = me.sprite, owner.sprite
        look.color_shader = worn.color_shader
        look.palette = worn.palette
        look.alpha = worn.alpha
        local s = me.state :: State
        if me.action == WAIT then
            -- EXE5's 0x080C41F4, EXE4's 0x080CCD98: every navi in, it stops running while paused.
            if battle.navis_in() then
                me.run_while_paused = false
                me:set_action(follows)
            end
            return
        end
        if me.action ~= follows then
            error(string.format("NumberMan's face: action %#x reads past its table", me.action))
        end
        -- EXE5's 0x080C4258, EXE4's 0x080CCECC: hidden while he moves; its animation from its start when he stands
        -- again.
        if owner.anim ~= 0 then
            me.visible = false
            s.hidden = true
        elseif s.hidden then
            s.hidden = false
            me.anim_loaded = 0xFF
        end
        me:update_sprite()
    end

    -- EXE5's 0x080C414C, EXE4's 0x080CCCBC: its sprite (no shadow) at animation 0, a frame on, facing its owner's
    -- way, in his palette; its start; then its first update.
    local function init(me: Object)
        local owner = me.related1 :: Object
        local sprite = me.sprite
        sprite:load(spec.sprite)
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = 0, 0
        sprite:set_animation(0)
        sprite:step()
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        sprite.palette = owner.sprite.palette
        me:set_lifecycle("update")
        local s = me.state :: State
        s.hidden = false
        if spec.follows_in_fight then
            me.run_while_paused = false
            if battle.fighting() then
                me.action = follows
            else
                me.run_while_paused = true
                me.action = WAIT
            end
        end
        update(me)
    end

    local kind = new.kind {
        id = spec.id,
        pool = "actor",
        state = { hidden = "bool" },
        -- EXE5's 0x080C412E, EXE4's 0x080CCC9C.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                update(me)
            else
                me:free()
            end
        end,
    }

    -- EXE5's 0x080C42C0, EXE4's 0x080CCF34: a face on `owner`, of his side and facing, running while paused.
    local function spawn(owner: Object): Object?
        local o = battle.spawn(kind, Vec3.zero)
        if o then
            o.related1 = owner
            o.alliance, o.flip = owner.alliance, owner.flip
            o.run_while_paused = true
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return face
