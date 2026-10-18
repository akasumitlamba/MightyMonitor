# Revised design: a connected desk

Anthropic's frontend-design skill is installed at ~/.codex/skills/frontend-design and was read before this redesign. The previous cosmetic revision was too close to the original split hero. This pass replaces the composition, visual hierarchy, use-case presentation, progress layout, and downloads.

## Plan

Palette: deep blue #102748 (hero), porcelain #f3f5fa (reading surface), periwinkle #a7baff (actions), mint #a1e7d4 (connected state), ink #182337 (body), slate #56657c (secondary text).
Typography: locally bundled Sora for display and IBM Plex Sans for body; no remote font service needed when viewing the files.
Layout: an expansive dark-blue opening with a large left-aligned headline, a concise right-hand purpose statement, and a full-width interactive three-device desk beneath. A pale reading surface follows. A vertical development rail replaces the old four-card roadmap; compact platform download rows replace the old download-card grid.

[ logo                      progress / downloads / source ]
[ One desk. Every device.                purpose + action ]
[       Linux monitor <-> Windows laptop <-> Android      ]
[         Mirror       Control       Extend              ]
[ purpose / use-case detail      vertical progress rail  ]
[ Downloads: Windows          availability / installers  ]
[            Linux            availability / installers  ]
[            Android          availability / installers  ]
[ practical questions                          footer   ]

## Critique before implementation

Keep the device relationship as the one memorable element, grounded in the actual goal of running Linux, Windows, and Android together. Avoid decorative step numbers on modes, all-caps section eyebrows, repetitive feature cards, and colored half-headlines. The timeline is a real implementation sequence, so its structure carries meaning.

The illustration is explicitly a concept; do not imply Android/control/extension features work. Downloads remain real release-asset links, not placeholders. All reading content and links work without JavaScript; JS adds the mode demo and live release discovery. No build process, application server, account, or hosted platform dependency. Respect file:// paths, GitHub Pages repository subpaths, reduced motion, keyboard focus, and narrow screens.

## Hosting decision

Local files and GitHub Pages only, as requested. No Sites manifest, source synchronization, or deployments. A previous private hosted preview is unrelated to this deliverable and is not updated.
