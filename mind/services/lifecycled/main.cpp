// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
#include "LifecycleService.h"
#include "cybou/fabric/OrganBus.h"
#include "cybou/fabric/ServiceHost.h"
#include "cybou/runtime/StatePaths.h"
#include <QCoreApplication>
#include <QDir>
#include <QTextStream>
int main(int argc,char **argv){QCoreApplication app(argc,argv);QCoreApplication::setApplicationName("cybou-lifecycled");
 auto path=QDir(cybou::StatePaths::persistentRoot()).filePath("lifecycle/state.json");cybou::LifecycleService service(path);
 if(!service.isReady()){QTextStream(stderr)<<service.startupError()<<Qt::endl;return 2;}QString error;
 if(!cybou::ServiceHost::publish(&service,cybou::kLifecycleEndpoint,&error)){QTextStream(stderr)<<error<<Qt::endl;return 3;}return app.exec();}
