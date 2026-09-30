--!strict
-- The content pack's data these scripts read: each chip's own data (by
-- chip id), the attachment kinds (by the attachment's first parameter) and
-- the sun beam's looks. It comes from the content pack a battle runs on:
-- the engine replaces this module with one built from the pack when it
-- loads the scripts (`bn6_battle::behavior::luau_pack`), frozen like every
-- module. This stub only declares its shape; game data is never part of
-- the repository.

return {
    chips = {} :: { [number]: ChipData },
    attachments = {} :: { [number]: AttachmentKind },
    sun_beam_looks = {} :: { [number]: SpriteId },
}
