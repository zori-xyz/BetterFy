//! Pinned event bindings.
//!
//! An event attribute in a Panorama layout (`onactivate="..."`) holds inline
//! code. A layout may carry one only if it is *exactly* one of these bindings:
//! the element type, its `id`, the attribute and the code, character for
//! character. Each was read in the layouts of real background and
//! versus-screen archives, where they are copies of the handlers of Valve's
//! own dashboard and versus layouts, and reviewed one by one: every call is a
//! Valve interface event or a function of a Valve script (`PlayScene()` of
//! the versus scripts); none opens a URL, reaches the network, runs a console
//! command or uses `$`. Pinning the element id as well keeps an author from
//! putting a handler on a button of their own.
//!
//! The list is reviewed by hand and compiled into the app; nothing outside
//! this file can extend it.

/// `(element, id, attribute, code)`.
const PINNED: &[(&str, &str, &str, &str)] = &[
    ("Button", "CurrentPartyVisibilityButton", "onactivate", "DOTAPartyVisibilityButtonClicked();"),
    ("Button", "CurrentPartyVisibilityButton", "onmouseout", "DOTAPartyVisibilityButtonMouseOut();"),
    ("Button", "CurrentPartyVisibilityButton", "onmouseover", "DOTAPartyVisibilityButtonMouseOver();"),
    ("Button", "DOTANotificationButton", "onmouseout", "AsyncEvent( 0.0, DOTASetNotificationsMenuVisible( false ) ); RemoveStyle( TopBarNotificationsHover, Hover )"),
    ("Button", "DOTANotificationButton", "onmouseover", "DOTASetNotificationsMenuVisible( true ); AddStyle( TopBarNotificationsHover, Hover ); UIHideTextTooltip();"),
    ("Button", "DOTAQuitButton", "onactivate", "DOTAQuit();"),
    ("Button", "DOTAQuitButton", "onmouseout", "RemoveStyle( TopBarExitHover, Hover )"),
    ("Button", "DOTAQuitButton", "onmouseover", "AddStyle( TopBarExitHover, Hover )"),
    ("Button", "DOTAReturnToGameButton", "onactivate", "DOTAReturnToGame();"),
    ("Button", "DOTASettingsButton", "onactivate", "DOTAShowSettingsPopup();"),
    ("Button", "DOTASettingsRebornButton", "onactivate", "DOTAShowSettingsRebornPopup();"),
    ("Button", "DashboardPagesBlocker", "onactivate", "DOTAClosePlayTab()"),
    ("Button", "FriendsButton", "onactivate", "DOTAFriendsButtonClicked();"),
    ("Button", "FriendsButton", "onmouseout", "DOTAFriendsButtonMouseOut();"),
    ("Button", "FriendsButton", "onmouseover", "DOTAFriendsButtonMouseOver();"),
    ("Button", "GuidesButton", "onactivate", "ToggleStyle( DashboardPagesGuides, Visible )"),
    ("Button", "GuildButton", "onactivate", "DOTAShowDefaultGuildPage()"),
    ("Button", "GuildButton", "onmouseout", "DOTARequestGuildStatusVisible( false )"),
    ("Button", "GuildButton", "onmouseover", "DOTARequestGuildStatusVisible( true ); DOTAAcknowledgeViolator( 'guilds_violator' )"),
    ("Button", "HideDevButtons", "onactivate", "AddStyle( TopBarRightButtons, HideButtons )"),
    ("Button", "NavigateBackButton", "onactivate", "DOTANavigateBack()"),
    ("Button", "NavigateForwardButton", "onactivate", "DOTANavigateForward()"),
    ("Button", "PartyOpenMic", "onactivate", "DOTAPartyOpenMicButtonClicked()"),
    ("Button", "PartyOpenMic", "onmouseout", "UIHideTextTooltip()"),
    ("Button", "PendingInvites", "onactivate", "DOTAShowPendingInvitesContextMenu()"),
    ("Button", "PendingInvites", "onmouseout", "DOTAHidePendingInvitesTooltip()"),
    ("Button", "PendingInvites", "onmouseover", "DOTAShowPendingInvitesTooltip()"),
    ("Button", "ReloadPageButton", "onactivate", "DOTAReloadCurrentPage()"),
    ("Button", "SearchButton", "onactivate", "DOTASearchTextSubmitted()"),
    ("Button", "ZooButton", "onactivate", "DOTAShowControlsLibraryPageTab( Default );"),
    ("DOTAChat", "Chat", "oncancel", "SetInputFocus( DashboardPages )"),
    ("DOTACoachingButton", "CoachingButton", "onmouseout", "DOTARequestCoachingMenuVisible( false )"),
    ("DOTACoachingButton", "CoachingButton", "onmouseover", "DOTARequestCoachingMenuVisible( true )"),
    ("DOTACoachingMenu", "CoachingMenu", "onmouseout", "DOTARequestCoachingMenuVisible( false )"),
    ("DOTACoachingMenu", "CoachingMenu", "onmouseover", "DOTARequestCoachingMenuVisible( true )"),
    ("DOTADashboardPopupManager", "DashboardPopupManager", "onpopupsdismissed", "SetInputFocus( DashboardPages )"),
    ("DOTAGuildStatus", "GuildStatus", "onmouseout", "DOTARequestGuildStatusVisible( false )"),
    ("DOTAGuildStatus", "GuildStatus", "onmouseover", "DOTARequestGuildStatusVisible( true )"),
    ("DOTAHelpTipsManager", "HelpTipsManager", "oncancel", "DropInputFocus()"),
    ("DOTAHomePage", "", "onnavigatedaway", "DOTASetFriendsMenuVisible( false )"),
    ("DOTAHomePage", "", "onnavigatedto", "DOTASetFriendsMenuVisible( true )"),
    ("DOTANotificationsMenu", "NotificationsMenu", "onmouseout", "AsyncEvent( 0.0, DOTASetNotificationsMenuVisible( false ) )"),
    ("DOTANotificationsMenu", "NotificationsMenu", "onmouseover", "DOTASetNotificationsMenuVisible( true )"),
    ("DOTAPlusButton", "TopBarPlus", "onmouseout", "AsyncEvent( 0.0, DOTASetPlusMenuVisible( false ) )"),
    ("DOTAPlusButton", "TopBarPlus", "onmouseover", "DOTASetPlusMenuVisible( true )"),
    ("DOTAPlusMenu", "PlusMenu", "onmouseout", "AsyncEvent( 0.0, DOTASetPlusMenuVisible( false ) )"),
    ("DOTAPlusMenu", "PlusMenu", "onmouseover", "DOTASetPlusMenuVisible( true )"),
    ("DOTARecentArmoryItemsMenu", "RecentArmoryItemsMenu", "onmouseout", "AsyncEvent( 0.0, DOTASetRecentArmoryItemsMenuVisible( false ) )"),
    ("DOTARecentArmoryItemsMenu", "RecentArmoryItemsMenu", "onmouseover", "DOTASetRecentArmoryItemsMenuVisible( true )"),
    ("DOTAScenePanel", "HomeButtonMap", "onload", "DOTASceneFireEntityInput( ui_home_embers, start, 1 )"),
    ("DOTAShardsButton", "ShardsButton", "onmouseout", "AsyncEvent( 0.0, DOTASetShardsMenuVisible( false ) )"),
    ("DOTAShardsButton", "ShardsButton", "onmouseover", "DOTASetShardsMenuVisible( true )"),
    ("DOTAShardsMenu", "ShardsMenu", "onmouseout", "AsyncEvent( 0.0, DOTASetShardsMenuVisible( false ) )"),
    ("DOTAShardsMenu", "ShardsMenu", "onmouseover", "DOTASetShardsMenuVisible( true )"),
    ("Panel", "PartyHasLargeRankSpreadTooltip", "onmouseout", "UIHideTextTooltip();"),
    ("Panel", "PartyHasLargeRankSpreadTooltip", "onmouseover", "UIShowTextTooltip( #CompetitiveMMLargeRankSpreadWarning );"),
    ("Panel", "SearchResults", "oncancel", "DOTASearchHideResults()"),
    ("Panel", "SearchResults", "onmoveup", "DOTASearchFocusText()"),
    ("RadioButton", "LeftTopBarPostGame", "onactivate", "DOTAReturnToPostGame();"),
    ("RadioButton", "TopBarArmory", "onactivate", "DOTAShowArmory();"),
    ("RadioButton", "TopBarArmory", "onmouseout", "AsyncEvent( 0.0, DOTASetRecentArmoryItemsMenuVisible( false ) ); RemoveStyle( TopBarArmoryHover, Hover )"),
    ("RadioButton", "TopBarArmory", "onmouseover", "DOTASetRecentArmoryItemsMenuVisible( true ); AddStyle( TopBarArmoryHover, Hover ); UIHideTextTooltip();"),
    ("RadioButton", "TopBarCollection", "onactivate", "DOTAShowPrimaryTabPage( TopBarCollection )"),
    ("RadioButton", "TopBarHeroes", "onactivate", "DOTAShowPrimaryTabPage( TopBarHeroes );"),
    ("RadioButton", "TopBarHome", "onactivate", "DOTAShowHomePage()"),
    ("RadioButton", "TopBarHome", "ondeselect", "DOTAGlobalSceneFireEntityInput( HomeButtonMap, ui_home_embers, stopplayendcap, 1 )"),
    ("RadioButton", "TopBarHome", "onload", "DOTAAttachHelpTip( 'npe_wizard_dota_logo' );"),
    ("RadioButton", "TopBarHome", "onselect", "DOTAGlobalSceneFireEntityInput( HomeButtonMap, ui_home_embers, start, 1 )"),
    ("RadioButton", "TopBarLearn", "onactivate", "DOTAShowPrimaryTabPage( TopBarLearn );"),
    ("RadioButton", "TopBarMods", "onactivate", "DOTAShowPrimaryTabPage( TopBarMods )"),
    ("RadioButton", "TopBarWatch", "onactivate", "DOTAShowPrimaryTabPage( TopBarWatch );"),
    ("RadioButton", "TopBarWatch", "onmouseout", "AsyncEvent( 0.0, DOTASetWatchMenuVisible( false ) );"),
    ("RadioButton", "TopBarWatch", "onmouseover", "DOTASetWatchMenuVisible( true );"),
    ("TextButton", "", "onactivate", "PlayScene()"),
    ("TextEntry", "SearchTextEntry", "oncancel", "DOTASearchHideResults()"),
    ("TextEntry", "SearchTextEntry", "onfocus", "DOTASearchTextFocused()"),
    ("TextEntry", "SearchTextEntry", "oninputsubmit", "DOTASearchTextSubmitted()"),
    ("TextEntry", "SearchTextEntry", "onmovedown", "DOTASearchFocusResults()"),
    ("ToggleButton", "ToggleFadeOut", "onactivate", "ToggleFadeOutEnabled()"),
];

pub(super) fn is_reviewed(element: &str, id: &str, attribute: &str, code: &str) -> bool {
    PINNED.contains(&(element, id, attribute, code))
}

#[cfg(test)]
pub(super) fn all() -> &'static [(&'static str, &'static str, &'static str, &'static str)] {
    PINNED
}
