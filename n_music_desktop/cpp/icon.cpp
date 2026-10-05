#include "icon.h"

#include <QtGui/QGuiApplication>
#include <QtGui/QIcon>

void
installWindowIcon()
{
  QGuiApplication::setWindowIcon(
    QIcon(QStringLiteral(":/qt/qml/NMusic/assets/icons/icon.png")));
}
